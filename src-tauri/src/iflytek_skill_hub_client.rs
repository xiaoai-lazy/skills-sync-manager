use crate::models::AppError;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HTTP_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IflytekSkillDto {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub namespace: String,
    pub latest_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClawHubWellKnown {
    #[serde(rename = "apiBase")]
    api_base: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SkillsListEnvelope {
    items: Option<Vec<RawIflytekSkill>>,
    results: Option<Vec<RawIflytekSkill>>,
}

#[derive(Debug, Deserialize)]
struct VersionObject {
    #[serde(default)]
    version: Option<String>,
}

/// Accepts either a plain version string or `{ "version": "..." }` (ClawHub / SkillHub).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FlexibleVersion {
    String(String),
    Object(VersionObject),
}

impl FlexibleVersion {
    fn into_string(self) -> Option<String> {
        match self {
            FlexibleVersion::String(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            FlexibleVersion::Object(obj) => obj
                .version
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawIflytekSkill {
    slug: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default, rename = "latestVersion")]
    latest_version: Option<FlexibleVersion>,
    #[serde(default)]
    version: Option<FlexibleVersion>,
}

pub fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

pub fn parse_canonical_slug(canonical: &str) -> (String, String) {
    if let Some((namespace, slug)) = canonical.split_once("--") {
        if !namespace.is_empty() && !slug.is_empty() {
            return (namespace.to_string(), slug.to_string());
        }
    }
    ("global".to_string(), canonical.to_string())
}

pub fn parse_skills_list(json: &str) -> Result<Vec<IflytekSkillDto>, AppError> {
    if let Ok(items) = serde_json::from_str::<Vec<RawIflytekSkill>>(json) {
        return Ok(items.into_iter().map(map_raw_skill).collect());
    }

    let envelope: SkillsListEnvelope = serde_json::from_str(json).map_err(|err| AppError::Io {
        path: None,
        message: format!("iFlytek Hub 请求失败: 解析 Skill 列表失败: {}", err),
    })?;

    let items = envelope
        .items
        .or(envelope.results)
        .unwrap_or_default();

    Ok(items.into_iter().map(map_raw_skill).collect())
}

fn map_raw_skill(raw: RawIflytekSkill) -> IflytekSkillDto {
    let (derived_namespace, derived_slug) = parse_canonical_slug(&raw.slug);
    let has_explicit_namespace = raw
        .namespace
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let namespace = raw
        .namespace
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(derived_namespace);
    let slug = if has_explicit_namespace {
        raw.slug
    } else {
        derived_slug
    };

    IflytekSkillDto {
        slug: slug.clone(),
        name: raw
            .display_name
            .or(raw.name)
            .unwrap_or_else(|| slug.clone()),
        description: raw.summary.or(raw.description).unwrap_or_default(),
        namespace,
        latest_version: raw
            .latest_version
            .and_then(FlexibleVersion::into_string)
            .or_else(|| raw.version.and_then(FlexibleVersion::into_string)),
    }
}

/// Join well-known `apiBase` against the endpoint base URL.
/// - Absolute `apiBase` (has scheme): normalize and return as-is
/// - Relative (starts with `/` or no scheme): join with endpoint origin/base
pub fn join_api_base(endpoint_base: &str, api_base: &str) -> String {
    let api_base = api_base.trim();
    if api_base.is_empty() {
        return format!("{}/api/v1", normalize_base_url(endpoint_base));
    }

    if api_base.contains("://") {
        return normalize_base_url(api_base);
    }

    let endpoint = normalize_base_url(endpoint_base);
    if api_base.starts_with('/') {
        let origin = endpoint_origin(&endpoint).unwrap_or(endpoint);
        return normalize_base_url(&format!("{}{}", origin, api_base));
    }

    normalize_base_url(&format!("{}/{}", endpoint, api_base))
}

fn endpoint_origin(endpoint: &str) -> Option<String> {
    let (scheme, rest) = endpoint.split_once("://")?;
    let host_port = rest.split('/').next().filter(|value| !value.is_empty())?;
    Some(format!("{}://{}", scheme, host_port))
}

pub fn resolve_api_base(base_url: &str) -> Result<String, AppError> {
    let base = normalize_base_url(base_url);
    let well_known_url = format!("{}/.well-known/clawhub.json", base);

    match send_request(blocking_client().get(&well_known_url)) {
        Ok(response) if response.status().as_u16() == 200 => {
            let body = read_response_body(&well_known_url, response)?;
            if let Ok(payload) = serde_json::from_str::<ClawHubWellKnown>(&body) {
                if let Some(api_base) = payload
                    .api_base
                    .filter(|value| !value.trim().is_empty())
                {
                    return Ok(join_api_base(&base, &api_base));
                }
            }
        }
        Ok(response) => {
            let _ = read_response_body(&well_known_url, response);
        }
        Err(_) => {}
    }

    Ok(format!("{}/api/v1", base))
}

pub fn fetch_skills(base_url: &str) -> Result<Vec<IflytekSkillDto>, AppError> {
    let api_base = resolve_api_base(base_url)?;
    let url = format!("{}/skills", normalize_base_url(&api_base));
    let response = send_request(blocking_client().get(&url))?;
    let status = response.status().as_u16();
    let body = read_response_body(&url, response)?;

    if status != 200 {
        return Err(map_iflytek_http_error(&url, status, &body));
    }

    parse_skills_list(&body)
}

pub fn download_skill_zip(
    base_url: &str,
    namespace: &str,
    slug: &str,
) -> Result<PathBuf, AppError> {
    let api_base = resolve_api_base(base_url)?;
    let url = format!(
        "{}/skills/{}/{}",
        normalize_base_url(&api_base),
        urlencoding::encode(namespace),
        urlencoding::encode(slug)
    );
    let download_url = format!("{}/download", url);
    let response = send_request(blocking_client().get(&download_url))?;
    let status = response.status().as_u16();

    if status != 200 {
        let body = read_response_body(&download_url, response).unwrap_or_default();
        return Err(map_iflytek_http_error(&download_url, status, &body));
    }

    let bytes = response.bytes().map_err(|err| AppError::Io {
        path: None,
        message: format!("iFlytek Hub 请求失败: {}", err),
    })?;

    let zip_path = create_temp_zip_path()?;
    fs::write(&zip_path, &bytes).map_err(|err| AppError::Io {
        path: Some(zip_path.clone()),
        message: format!("iFlytek Hub 请求失败: {}", err),
    })?;

    Ok(zip_path)
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
        message: format!("iFlytek Hub 请求失败: {}", err),
    })
}

fn read_response_body(
    url: &str,
    response: reqwest::blocking::Response,
) -> Result<String, AppError> {
    response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("iFlytek Hub 请求失败 {}: {}", url, err),
    })
}

#[derive(Deserialize)]
struct IflytekErrorBody {
    error: String,
}

fn iflytek_error_message(body: &str, fallback: &str) -> String {
    serde_json::from_str::<IflytekErrorBody>(body)
        .ok()
        .map(|payload| payload.error)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn map_iflytek_http_error(url: &str, status: u16, body: &str) -> AppError {
    if status == 404 {
        if let Some((group, skill_id)) = parse_download_path_ids(url) {
            return AppError::HubSkillGone { skill_id, group };
        }
    }
    AppError::DownloadFailed {
        url: url.to_string(),
        status: Some(status),
        message: iflytek_error_message(
            body,
            &format!("iFlytek Hub 请求失败，HTTP 状态码 {}", status),
        ),
    }
}

/// `/skills/{namespace}/{slug}/download`
fn parse_download_path_ids(url: &str) -> Option<(String, String)> {
    let path = url.split('?').next().unwrap_or(url);
    let without_download = path
        .strip_suffix("/download")
        .or_else(|| path.strip_suffix("/download/"))?;
    let marker = "/skills/";
    let start = without_download.rfind(marker)? + marker.len();
    let rest = &without_download[start..];
    let (namespace_enc, slug_enc) = rest.split_once('/')?;
    if namespace_enc.is_empty() || slug_enc.is_empty() || slug_enc.contains('/') {
        return None;
    }
    let group = urlencoding::decode(namespace_enc)
        .ok()
        .map(|c| c.into_owned())
        .unwrap_or_else(|| namespace_enc.to_string());
    let skill_id = urlencoding::decode(slug_enc)
        .ok()
        .map(|c| c.into_owned())
        .unwrap_or_else(|| slug_enc.to_string());
    Some((group, skill_id))
}

fn create_temp_zip_path() -> Result<PathBuf, AppError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("iFlytek Hub 请求失败: {}", err),
        })?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("skills-sync-iflytek-skill-{}.zip", nanos)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skills_list_maps_items_and_global_slug() {
        let json = r#"{"items":[{"slug":"task-decomposition","displayName":"task-decomposition","summary":"x","latestVersion":"1"}]}"#;
        let list = parse_skills_list(json).unwrap();
        assert_eq!(list[0].namespace, "global");
        assert_eq!(list[0].slug, "task-decomposition");
        assert_eq!(list[0].latest_version.as_deref(), Some("1"));
    }

    #[test]
    fn parse_skills_list_accepts_latest_version_object() {
        let json = r#"{
          "items":[{
            "slug":"task-decomposition",
            "displayName":"task-decomposition",
            "summary":"x",
            "latestVersion":{"version":"20260408.063033","createdAt":1,"changelog":"","license":null}
          }],
          "nextCursor":null
        }"#;
        let list = parse_skills_list(json).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].slug, "task-decomposition");
        assert_eq!(list[0].namespace, "global");
        assert_eq!(list[0].latest_version.as_deref(), Some("20260408.063033"));
    }

    #[test]
    fn parse_canonical_slug_team() {
        assert_eq!(parse_canonical_slug("ued--foo"), ("ued".into(), "foo".into()));
    }

    #[test]
    fn parse_skills_list_results_envelope() {
        let json = r#"{"results":[{"slug":"ued--bar","name":"bar","description":"d"}]}"#;
        let list = parse_skills_list(json).unwrap();
        assert_eq!(list[0].namespace, "ued");
        assert_eq!(list[0].slug, "bar");
    }

    #[test]
    fn join_api_base_joins_absolute_path_against_endpoint() {
        assert_eq!(
            join_api_base("https://skillhub.xkw.cn", "/api/v1"),
            "https://skillhub.xkw.cn/api/v1"
        );
        assert_eq!(
            join_api_base("https://skillhub.xkw.cn/", "/api/v1/"),
            "https://skillhub.xkw.cn/api/v1"
        );
    }

    #[test]
    fn join_api_base_joins_scheme_less_relative_path() {
        assert_eq!(
            join_api_base("https://skillhub.xkw.cn", "api/v1"),
            "https://skillhub.xkw.cn/api/v1"
        );
    }

    #[test]
    fn join_api_base_keeps_absolute_url_with_scheme() {
        assert_eq!(
            join_api_base(
                "https://skillhub.xkw.cn",
                "https://cdn.example.com/hub-api/v1"
            ),
            "https://cdn.example.com/hub-api/v1"
        );
    }

    #[test]
    fn map_iflytek_http_error_maps_download_404_to_hub_skill_gone() {
        let err = map_iflytek_http_error(
            "https://skillhub.xkw.cn/api/v1/skills/global/task-decomposition/download",
            404,
            r#"{"error":"Skill 不存在"}"#,
        );
        assert!(matches!(
            err,
            AppError::HubSkillGone {
                ref skill_id,
                ref group,
            } if skill_id == "task-decomposition" && group == "global"
        ));
        let dto = err.to_dto();
        assert_eq!(dto.code, "hubSkillGone");
    }

    #[test]
    fn parse_download_path_ids_decodes_url_components() {
        let parsed = parse_download_path_ids(
            "https://hub/api/v1/skills/my%20ns/skill%2Fid/download",
        );
        assert_eq!(
            parsed,
            Some(("my ns".to_string(), "skill/id".to_string()))
        );
    }
}
