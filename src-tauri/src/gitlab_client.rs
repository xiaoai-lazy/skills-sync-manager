use crate::models::AppError;
use crate::skill_downloader;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

pub fn archive_url(host: &str, project_path: &str, branch: &str) -> String {
    format!(
        "https://{}/api/v4/projects/{}/repository/archive.zip?sha={}",
        normalize_host(host),
        urlencoding::encode(project_path),
        urlencoding::encode(branch)
    )
}

pub fn api_url(host: &str, path: &str) -> String {
    let path = path.trim_start_matches('/');
    format!("https://{}/api/v4/{}", normalize_host(host), path)
}

pub fn fetch_default_branch(
    host: &str,
    project_path: &str,
    token: &str,
) -> Result<String, AppError> {
    let url = format!(
        "https://{}/api/v4/projects/{}",
        normalize_host(host),
        urlencoding::encode(project_path)
    );
    let response = send_request(blocking_client().get(&url), Some(token))?;
    let status = response.status().as_u16();

    if matches!(status, 401 | 403) {
        return Err(map_auth_error(host, Some(token)));
    }

    if status == 404 {
        let (owner, name) = project_path_to_repo_parts(project_path);
        return Err(AppError::SkillRepoNotFound { owner, name });
    }

    if status != 200 {
        return Err(AppError::DownloadFailed {
            url: url.clone(),
            status: Some(status),
            message: format!("读取 GitLab 项目信息失败，HTTP 状态码 {}", status),
        });
    }

    let body = response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("读取 GitLab 项目信息失败 {}: {}", url, err),
    })?;

    let project: GitLabProject = serde_json::from_str(&body).map_err(|err| AppError::Io {
        path: None,
        message: format!("解析 GitLab 项目信息失败 {}: {}", url, err),
    })?;

    project.default_branch.ok_or_else(|| AppError::Io {
        path: None,
        message: format!(
            "GitLab 项目 {} 未返回 default_branch 字段",
            project_path
        ),
    })
}

pub fn validate_token(host: &str, token: &str) -> Result<(), AppError> {
    let url = api_url(host, "user");
    let response = send_request(blocking_client().get(&url), Some(token))?;

    match response.status().as_u16() {
        200 => Ok(()),
        401 | 403 => Err(AppError::GitLabAuthInvalid {
            host: normalize_host(host),
        }),
        status => Err(AppError::DownloadFailed {
            url,
            status: Some(status),
            message: format!("验证 GitLab Token 失败，HTTP 状态码 {}", status),
        }),
    }
}

pub fn probe_instance(host: &str) -> Result<(), AppError> {
    let url = api_url(host, "version");
    let response = send_request(blocking_client().get(&url), None)?;
    classify_instance_probe_response(host, &url, response.status().as_u16())
}

/// 判断 GitLab 实例 API 是否可达（可单测）。
pub fn classify_instance_probe_response(
    host: &str,
    url: &str,
    status: u16,
) -> Result<(), AppError> {
    match status {
        // 部分自建 GitLab（如 gitlab.example.com）对 /version 也要求登录，401/403 仍表示 API 端点存在
        200 | 401 | 403 => Ok(()),
        404 => Err(AppError::DownloadFailed {
            url: url.to_string(),
            status: Some(status),
            message: format!(
                "地址 {} 未提供 GitLab API（/api/v4/version 不存在），请检查站点地址",
                normalize_host(host)
            ),
        }),
        _ => Err(AppError::DownloadFailed {
            url: url.to_string(),
            status: Some(status),
            message: format!(
                "无法连接 GitLab 站点 {}，HTTP 状态码 {}",
                normalize_host(host),
                status
            ),
        }),
    }
}

pub fn probe_project_access(
    host: &str,
    project_path: &str,
    token: Option<&str>,
) -> Result<(), AppError> {
    let url = format!(
        "https://{}/api/v4/projects/{}/repository/archive.zip",
        normalize_host(host),
        urlencoding::encode(project_path)
    );
    let response = send_request(blocking_client().head(&url), token)?;

    map_project_access_response(host, project_path, token, &url, response.status().as_u16())
}

pub fn fetch_file_raw(
    host: &str,
    project_path: &str,
    file_path: &str,
    branch: &str,
    token: Option<&str>,
) -> Result<String, AppError> {
    let url = format!(
        "https://{}/api/v4/projects/{}/repository/files/{}/raw?ref={}",
        normalize_host(host),
        urlencoding::encode(project_path),
        urlencoding::encode(file_path),
        urlencoding::encode(branch)
    );
    let response = send_request(blocking_client().get(&url), token)?;
    let status = response.status().as_u16();

    if status == 200 {
        return response.text().map_err(|err| AppError::Io {
            path: None,
            message: format!("读取 GitLab 文件内容失败 {}: {}", url, err),
        });
    }

    if matches!(status, 401 | 403) {
        return Err(map_auth_error(host, token));
    }

    if status == 404 {
        return Err(AppError::Io {
            path: None,
            message: format!(
                "GitLab 文件不存在：{}/{}@{}",
                project_path, file_path, branch
            ),
        });
    }

    Err(AppError::DownloadFailed {
        url,
        status: Some(status),
        message: format!("读取 GitLab 文件失败，HTTP 状态码 {}", status),
    })
}

pub fn download_archive(
    host: &str,
    project_path: &str,
    branch: &str,
    token: Option<&str>,
) -> Result<PathBuf, AppError> {
    let url = archive_url(host, project_path, branch);
    let mut headers: Vec<(&str, &str)> = Vec::new();
    if let Some(token) = token {
        headers.push(("PRIVATE-TOKEN", token));
    }

    match skill_downloader::download_and_extract_with_headers(&url, &headers) {
        Ok(path) => Ok(path),
        Err(AppError::DownloadFailed {
            status: Some(401 | 403),
            ..
        }) => Err(map_auth_error(host, token)),
        Err(err) => {
            if let AppError::DownloadFailed {
                status: Some(404),
                ..
            } = &err
            {
                let (owner, name) = project_path_to_repo_parts(project_path);
                return Err(AppError::SkillRepoNotFound { owner, name });
            }
            Err(err)
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    default_branch: Option<String>,
}

const HTTP_TIMEOUT_SECS: u64 = 60;

fn blocking_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

fn normalize_host(host: &str) -> String {
    host.trim().trim_end_matches('/').to_string()
}

fn send_request(
    builder: reqwest::blocking::RequestBuilder,
    token: Option<&str>,
) -> Result<reqwest::blocking::Response, AppError> {
    let builder = if let Some(token) = token {
        builder.header("PRIVATE-TOKEN", token)
    } else {
        builder
    };

    builder.send().map_err(|err| AppError::Io {
        path: None,
        message: format!("GitLab 请求失败: {}", err),
    })
}

fn map_auth_error(host: &str, token: Option<&str>) -> AppError {
    if token.is_some() {
        AppError::GitLabAuthInvalid {
            host: normalize_host(host),
        }
    } else {
        AppError::GitLabAuthRequired {
            host: normalize_host(host),
        }
    }
}

fn map_project_access_response(
    host: &str,
    project_path: &str,
    token: Option<&str>,
    url: &str,
    status: u16,
) -> Result<(), AppError> {
    match status {
        200 => Ok(()),
        401 | 403 => Err(map_auth_error(host, token)),
        404 => {
            let (owner, name) = project_path_to_repo_parts(project_path);
            Err(AppError::SkillRepoNotFound { owner, name })
        }
        _ => Err(AppError::DownloadFailed {
            url: url.to_string(),
            status: Some(status),
            message: format!("探测 GitLab 项目访问权限失败，HTTP 状态码 {}", status),
        }),
    }
}

fn project_path_to_repo_parts(project_path: &str) -> (String, String) {
    match project_path.rsplit_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (String::new(), project_path.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_url_encodes_project_path() {
        let url = archive_url("gitlab.example.com", "group/subgroup/project", "main");
        assert_eq!(
            url,
            "https://gitlab.example.com/api/v4/projects/group%2Fsubgroup%2Fproject/repository/archive.zip?sha=main"
        );
    }

    #[test]
    fn api_url_builds_correctly() {
        let url = api_url("gitlab.example.com", "user");
        assert_eq!(url, "https://gitlab.example.com/api/v4/user");
    }

    #[test]
    fn api_url_strips_leading_slash_from_path() {
        let url = api_url("gitlab.example.com", "/projects/1");
        assert_eq!(url, "https://gitlab.example.com/api/v4/projects/1");
    }

    #[test]
    fn classify_instance_probe_accepts_auth_required_as_reachable() {
        assert!(classify_instance_probe_response(
            "gitlab.example.com",
            "https://gitlab.example.com/api/v4/version",
            401
        )
        .is_ok());
        assert!(classify_instance_probe_response(
            "gitlab.example.com",
            "https://gitlab.example.com/api/v4/version",
            403
        )
        .is_ok());
    }

    #[test]
    fn classify_instance_probe_rejects_missing_api() {
        let err = classify_instance_probe_response(
            "example.com",
            "https://example.com/api/v4/version",
            404,
        )
        .expect_err("404 should fail");
        assert!(matches!(err, AppError::DownloadFailed { status: Some(404), .. }));
    }

    #[test]
    fn probe_instance_url_uses_version_endpoint() {
        let url = api_url("gitlab.example.com", "version");
        assert_eq!(url, "https://gitlab.example.com/api/v4/version");
    }

    #[test]
    fn project_path_to_repo_parts_splits_on_last_slash() {
        assert_eq!(
            project_path_to_repo_parts("group/sub/project"),
            ("group/sub".to_string(), "project".to_string())
        );
        assert_eq!(
            project_path_to_repo_parts("solo"),
            (String::new(), "solo".to_string())
        );
    }

    #[test]
    fn parse_gitlab_project_default_branch_json() {
        let body = r#"{"default_branch":"master"}"#;
        let project: super::GitLabProject = serde_json::from_str(body).expect("parse");
        assert_eq!(project.default_branch.as_deref(), Some("master"));
    }
}
