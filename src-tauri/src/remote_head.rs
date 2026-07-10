use crate::models::{default_github_host, AppError, RepoRef};
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

const HTTP_TIMEOUT_SECS: u64 = 30;

pub fn fetch_remote_head_sha(repo: &RepoRef) -> Result<String, AppError> {
    if repo.provider == "gitlab" || repo.host != default_github_host() {
        fetch_gitlab_head_sha(repo)
    } else {
        fetch_github_head_sha(repo)
    }
}

fn fetch_github_head_sha(repo: &RepoRef) -> Result<String, AppError> {
    let (owner, name) = repo
        .project_path
        .split_once('/')
        .ok_or_else(|| AppError::InvalidInput {
            input: repo.project_path.clone(),
            message: "GitHub 项目路径必须为 owner/name".to_string(),
        })?;

    let url = format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        owner,
        name,
        urlencoding::encode(&repo.branch)
    );

    let response = blocking_http_client()
        .get(&url)
        .header(reqwest::header::USER_AGENT, "skills-sync-manager")
        .send()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("获取 GitHub commit SHA 失败 {}: {}", url, err),
        })?;

    let status = response.status().as_u16();
    let body = response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("读取 GitHub commit 响应失败 {}: {}", url, err),
    })?;

    if status == 404 {
        let (owner, name) = parse_github_project_path(&repo.project_path)?;
        return Err(AppError::DownloadFailed {
            url: url.clone(),
            status: Some(status),
            message: format!(
                "仓库 {}/{} 的分支 '{}' 不存在",
                owner, name, repo.branch
            ),
        });
    }

    if status == 403 {
        return Err(AppError::DownloadFailed {
            url,
            status: Some(status),
            message: "GitHub 请求受限，请稍后再试".to_string(),
        });
    }

    if !(200..300).contains(&status) {
        return Err(AppError::DownloadFailed {
            url,
            status: Some(status),
            message: format!("获取 GitHub commit SHA 失败，HTTP 状态码 {}", status),
        });
    }

    parse_github_commit_sha(&body).map_err(|message| AppError::Io {
        path: None,
        message: format!("解析 GitHub commit 响应失败 {}: {}", url, message),
    })
}

fn fetch_gitlab_head_sha(repo: &RepoRef) -> Result<String, AppError> {
    let token = crate::credential_store::get_gitlab_token(&repo.host)?;
    let Some(token) = token else {
        return Err(AppError::GitLabAuthRequired {
            host: normalize_host(&repo.host),
        });
    };

    let url = format!(
        "https://{}/api/v4/projects/{}/repository/commits?ref_name={}&per_page=1",
        normalize_host(&repo.host),
        urlencoding::encode(&repo.project_path),
        urlencoding::encode(&repo.branch)
    );

    let response = blocking_http_client()
        .get(&url)
        .header("PRIVATE-TOKEN", &token)
        .send()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("获取 GitLab commit SHA 失败 {}: {}", url, err),
        })?;

    let status = response.status().as_u16();
    let body = response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("读取 GitLab commit 响应失败 {}: {}", url, err),
    })?;

    if matches!(status, 401 | 403) {
        return Err(AppError::GitLabAuthInvalid {
            host: normalize_host(&repo.host),
        });
    }

    if status == 404 {
        let (owner, name) = project_path_to_repo_parts(&repo.project_path);
        return Err(AppError::SkillRepoNotFound { owner, name });
    }

    if !(200..300).contains(&status) {
        return Err(AppError::DownloadFailed {
            url,
            status: Some(status),
            message: format!("获取 GitLab commit SHA 失败，HTTP 状态码 {}", status),
        });
    }

    parse_gitlab_commits_sha(&body).map_err(|message| AppError::Io {
        path: None,
        message: format!("解析 GitLab commit 响应失败 {}: {}", url, message),
    })
}

pub(crate) fn parse_github_commit_sha(body: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct GithubCommit {
        sha: String,
    }

    let commit: GithubCommit =
        serde_json::from_str(body).map_err(|err| format!("invalid JSON: {}", err))?;

    if commit.sha.is_empty() {
        return Err("missing sha field".to_string());
    }

    Ok(commit.sha)
}

pub(crate) fn parse_gitlab_commits_sha(body: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct GitLabCommit {
        id: String,
    }

    let commits: Vec<GitLabCommit> =
        serde_json::from_str(body).map_err(|err| format!("invalid JSON: {}", err))?;

    commits
        .first()
        .map(|commit| commit.id.clone())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "empty commits list".to_string())
}

fn parse_github_project_path(project_path: &str) -> Result<(&str, &str), AppError> {
    project_path.split_once('/').ok_or_else(|| AppError::InvalidInput {
        input: project_path.to_string(),
        message: "GitHub 项目路径必须为 owner/name 格式".to_string(),
    })
}

fn project_path_to_repo_parts(project_path: &str) -> (String, String) {
    match project_path.rsplit_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (project_path.to_string(), project_path.to_string()),
    }
}

fn normalize_host(host: &str) -> String {
    host.trim().trim_end_matches('/').to_string()
}

fn blocking_http_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_commit_sha_reads_sha_field() {
        let body = r#"{"sha":"abc123def","commit":{}}"#;
        assert_eq!(
            parse_github_commit_sha(body).expect("parse github sha"),
            "abc123def"
        );
    }

    #[test]
    fn parse_github_commit_sha_rejects_missing_sha() {
        let body = r#"{"commit":{}}"#;
        assert!(parse_github_commit_sha(body).is_err());
    }

    #[test]
    fn parse_gitlab_commits_sha_reads_first_commit_id() {
        let body = r#"[{"id":"deadbeef"}]"#;
        assert_eq!(
            parse_gitlab_commits_sha(body).expect("parse gitlab sha"),
            "deadbeef"
        );
    }

    #[test]
    fn parse_gitlab_commits_sha_rejects_empty_list() {
        assert!(parse_gitlab_commits_sha("[]").is_err());
    }

    #[test]
    fn fetch_remote_head_sha_routes_gitlab_provider() {
        let repo = RepoRef {
            host: "gitlab.example.com".to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/project".to_string(),
            branch: "main".to_string(),
        };
        let error = fetch_remote_head_sha(&repo).expect_err("gitlab without token should fail");
        assert!(matches!(error, AppError::GitLabAuthRequired { .. }));
    }
}
