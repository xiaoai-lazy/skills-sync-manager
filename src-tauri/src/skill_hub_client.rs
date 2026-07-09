use crate::models::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HTTP_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HubSkillDto {
    pub id: String,
    pub group: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HubGroupDto {
    pub name: String,
}

#[derive(Serialize)]
struct CreateGroupRequest<'a> {
    name: &'a str,
}

pub fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

pub fn api_url(base_url: &str, path: &str) -> String {
    format!("{}{}", normalize_base_url(base_url), path)
}

pub fn fetch_groups(base_url: &str) -> Result<Vec<HubGroupDto>, AppError> {
    let url = api_url(base_url, "/api/v1/groups");
    let response = send_request(blocking_client().get(&url))?;
    let status = response.status().as_u16();
    let body = read_response_body(&url, response)?;

    if status != 200 {
        return Err(map_hub_http_error(&url, status, &body));
    }

    parse_groups_list(&body)
}

pub fn create_group(base_url: &str, name: &str) -> Result<(), AppError> {
    let url = api_url(base_url, "/api/v1/groups");
    let response = send_request(
        blocking_client()
            .post(&url)
            .json(&CreateGroupRequest { name }),
    )?;
    let status = response.status().as_u16();
    let body = read_response_body(&url, response).unwrap_or_default();

    if (200..300).contains(&status) {
        return Ok(());
    }

    Err(map_hub_http_error(&url, status, &body))
}

pub fn fetch_skills(base_url: &str, group: Option<&str>) -> Result<Vec<HubSkillDto>, AppError> {
    let url = match group.filter(|value| !value.trim().is_empty()) {
        Some(group) => format!(
            "{}?group={}",
            api_url(base_url, "/api/v1/skills"),
            urlencoding::encode(group)
        ),
        None => api_url(base_url, "/api/v1/skills"),
    };
    let response = send_request(blocking_client().get(&url))?;
    let status = response.status().as_u16();
    let body = read_response_body(&url, response)?;

    if status != 200 {
        return Err(map_hub_http_error(&url, status, &body));
    }

    parse_skills_list(&body)
}

pub fn download_archive(
    base_url: &str,
    group: &str,
    skill_id: &str,
) -> Result<PathBuf, AppError> {
    let url = api_url(
        base_url,
        &format!(
            "/api/v1/groups/{}/skills/{}/archive",
            urlencoding::encode(group),
            urlencoding::encode(skill_id)
        ),
    );
    let response = send_request(blocking_client().get(&url))?;
    let status = response.status().as_u16();

    if status != 200 {
        let body = read_response_body(&url, response).unwrap_or_default();
        return Err(map_hub_http_error(&url, status, &body));
    }

    let bytes = response.bytes().map_err(|err| AppError::Io {
        path: None,
        message: format!("Hub 请求失败: {}", err),
    })?;

    let zip_path = create_temp_zip_path()?;
    fs::write(&zip_path, &bytes).map_err(|err| AppError::Io {
        path: Some(zip_path.clone()),
        message: format!("Hub 请求失败: {}", err),
    })?;

    Ok(zip_path)
}

pub fn upload_skill(
    base_url: &str,
    group: &str,
    skill_id: &str,
    archive_path: &Path,
) -> Result<(), AppError> {
    let url = api_url(base_url, "/api/v1/skills");
    let form = reqwest::blocking::multipart::Form::new()
        .text("id", skill_id.to_string())
        .text("group", group.to_string())
        .file("files[]", archive_path)
        .map_err(|err| AppError::Io {
            path: Some(archive_path.to_path_buf()),
            message: format!("上传失败: {}", err),
        })?;

    let response = send_upload_request(blocking_client().post(&url).multipart(form))?;
    let status = response.status().as_u16();
    let body = read_response_body(&url, response).unwrap_or_default();

    if (200..300).contains(&status) {
        return Ok(());
    }

    Err(map_upload_http_error(&url, status, &body))
}

pub fn parse_skills_list(json: &str) -> Result<Vec<HubSkillDto>, AppError> {
    serde_json::from_str(json).map_err(|err| AppError::Io {
        path: None,
        message: format!("Hub 请求失败: 解析 Skill 列表失败: {}", err),
    })
}

pub fn parse_groups_list(json: &str) -> Result<Vec<HubGroupDto>, AppError> {
    serde_json::from_str(json).map_err(|err| AppError::Io {
        path: None,
        message: format!("Hub 请求失败: 解析分组列表失败: {}", err),
    })
}

fn blocking_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

fn send_request(
    builder: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::Response, AppError> {
    builder.send().map_err(|err| AppError::Io {
        path: None,
        message: format!("Hub 请求失败: {}", err),
    })
}

fn send_upload_request(
    builder: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::Response, AppError> {
    builder.send().map_err(|err| AppError::Io {
        path: None,
        message: format!("上传失败: {}", err),
    })
}

fn read_response_body(
    url: &str,
    response: reqwest::blocking::Response,
) -> Result<String, AppError> {
    response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("Hub 请求失败 {}: {}", url, err),
    })
}

#[derive(Deserialize)]
struct HubErrorBody {
    error: String,
}

fn hub_error_message(body: &str, fallback: &str) -> String {
    serde_json::from_str::<HubErrorBody>(body)
        .ok()
        .map(|payload| payload.error)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn map_hub_http_error(url: &str, status: u16, body: &str) -> AppError {
    if status == 404 {
        if let Some((group, skill_id)) = parse_archive_path_ids(url) {
            return AppError::HubSkillGone { skill_id, group };
        }
    }
    AppError::DownloadFailed {
        url: url.to_string(),
        status: Some(status),
        message: hub_error_message(body, &format!("Hub 请求失败，HTTP 状态码 {}", status)),
    }
}

/// `/api/v1/groups/{group}/skills/{skill_id}/archive`
fn parse_archive_path_ids(url: &str) -> Option<(String, String)> {
    let path = url.split('?').next().unwrap_or(url);
    let marker = "/api/v1/groups/";
    let start = path.find(marker)? + marker.len();
    let rest = &path[start..];
    let (group_enc, after_group) = rest.split_once("/skills/")?;
    let skill_enc = after_group.strip_suffix("/archive").unwrap_or(after_group);
    let skill_enc = skill_enc.trim_end_matches('/');
    if group_enc.is_empty() || skill_enc.is_empty() {
        return None;
    }
    let group = urlencoding::decode(group_enc)
        .ok()
        .map(|c| c.into_owned())
        .unwrap_or_else(|| group_enc.to_string());
    let skill_id = urlencoding::decode(skill_enc)
        .ok()
        .map(|c| c.into_owned())
        .unwrap_or_else(|| skill_enc.to_string());
    Some((group, skill_id))
}

fn map_upload_http_error(url: &str, status: u16, body: &str) -> AppError {
    AppError::DownloadFailed {
        url: url.to_string(),
        status: Some(status),
        message: hub_error_message(body, &format!("上传失败，HTTP 状态码 {}", status)),
    }
}

fn create_temp_zip_path() -> Result<PathBuf, AppError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("Hub 请求失败: {}", err),
        })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("skills-sync-hub-archive-{}.zip", nanos)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skills_response_maps_group_to_hub_skill_group() {
        let json = r#"[{"id":"tdd","group":"common","name":"TDD","description":"Test"}]"#;
        let skills = parse_skills_list(json).unwrap();
        assert_eq!(skills[0].id, "tdd");
        assert_eq!(skills[0].group, "common");
    }

    #[test]
    fn parse_groups_list_works() {
        let json = r#"[{"name":"common"},{"name":"frontend"}]"#;
        let groups = parse_groups_list(json).unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "common");
        assert_eq!(groups[1].name, "frontend");
    }

    #[test]
    fn normalize_base_url_trims_trailing_slash() {
        assert_eq!(
            normalize_base_url("https://hub.example.com/"),
            "https://hub.example.com"
        );
        assert_eq!(
            normalize_base_url("https://hub.example.com"),
            "https://hub.example.com"
        );
    }

    #[test]
    fn api_url_joins_base_and_path() {
        assert_eq!(
            api_url("https://hub.example.com/", "/api/health"),
            "https://hub.example.com/api/health"
        );
    }

    #[test]
    fn parse_skills_list_accepts_optional_hash() {
        let json = r#"[{"id":"tdd","group":"common","name":"TDD","description":"Test","hash":"abc123"}]"#;
        let skills = parse_skills_list(json).unwrap();
        assert_eq!(skills[0].hash.as_deref(), Some("abc123"));
    }

    #[test]
    fn hub_error_message_parses_json_body() {
        let message = hub_error_message(r#"{"error":"分组名称不能为空"}"#, "fallback");
        assert_eq!(message, "分组名称不能为空");
    }

    #[test]
    fn map_hub_http_error_maps_archive_404_to_hub_skill_gone() {
        let err = map_hub_http_error(
            "http://127.0.0.1:3337/api/v1/groups/tools/skills/brainstorming/archive",
            404,
            r#"{"error":"Skill 不存在"}"#,
        );
        assert!(matches!(
            err,
            AppError::HubSkillGone {
                ref skill_id,
                ref group,
            } if skill_id == "brainstorming" && group == "tools"
        ));
        let dto = err.to_dto();
        assert_eq!(dto.code, "hubSkillGone");
        assert!(dto.message.contains("brainstorming"));
        assert!(dto.message.contains("源中已不存在"));
        assert!(dto.message.contains("tools"));
    }

    #[test]
    fn parse_archive_path_ids_decodes_url_components() {
        let parsed = parse_archive_path_ids(
            "http://hub/api/v1/groups/my%20group/skills/skill%2Fid/archive",
        );
        assert_eq!(
            parsed,
            Some(("my group".to_string(), "skill/id".to_string()))
        );
    }
}
