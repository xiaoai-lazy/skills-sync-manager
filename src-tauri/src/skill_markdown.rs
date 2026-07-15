use crate::gitlab_client;
use crate::models::{
    AppConfig, AppError, DiscoverableSkill, SkillMarkdownPreviewDto, SkillMarkdownRequestDto,
};
use crate::repo_cache;
use crate::skill_downloader;
use crate::skill_hub_client;
use crate::skill_hub_endpoints;
use crate::skill_install;
use crate::skill_library;
use crate::skill_storage;
use crate::credential_store;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMarkdownParts {
    pub title: Option<String>,
    pub description: Option<String>,
    pub markdown_body: String,
}

pub fn parse_skill_markdown_preview(raw: &str) -> SkillMarkdownParts {
    let (metadata, markdown_body) = skill_library::split_skill_md(raw);
    SkillMarkdownParts {
        title: metadata
            .as_ref()
            .and_then(|meta| trim_non_empty(meta.name.clone())),
        description: metadata
            .as_ref()
            .and_then(|meta| trim_non_empty(meta.description.clone())),
        markdown_body,
    }
}

pub fn read_installed_skill_markdown(
    main_skills_dir: Option<&Path>,
    storage_key: &str,
) -> Result<SkillMarkdownPreviewDto, AppError> {
    let skills = skill_library::list_skills(main_skills_dir)?;
    let skill = skills.iter().find(|s| s.storage_key == storage_key);

    let (skill_md_path, fallback_title, fallback_description) = if let Some(skill) = skill {
        (
            skill.path.join("SKILL.md"),
            skill.dir_name.clone(),
            skill.description.clone(),
        )
    } else if let Some(main_dir) = main_skills_dir {
        let skill_dir = skill_storage::main_library_path(main_dir, storage_key);
        let skill_md_path = skill_dir.join("SKILL.md");
        if !skill_md_path.is_file() {
            return Err(AppError::Io {
                path: None,
                message: format!("未找到 skill：{storage_key}"),
            });
        }
        (
            skill_md_path,
            skill_storage::skill_id_from_directory(storage_key),
            None,
        )
    } else {
        return Err(AppError::Io {
            path: None,
            message: format!("未找到 skill：{storage_key}"),
        });
    };

    let raw = fs::read_to_string(&skill_md_path).map_err(|err| AppError::Io {
        path: Some(skill_md_path.clone()),
        message: err.to_string(),
    })?;
    let parts = parse_skill_markdown_preview(&raw);

    Ok(SkillMarkdownPreviewDto {
        title: parts.title.unwrap_or(fallback_title),
        description: parts
            .description
            .or(fallback_description)
            .unwrap_or_default(),
        markdown_body: parts.markdown_body,
        origin: "mainLibrary".into(),
    })
}

pub fn read_skill_markdown(
    config: &AppConfig,
    app_data_dir: &Path,
    request: SkillMarkdownRequestDto,
) -> Result<SkillMarkdownPreviewDto, AppError> {
    read_skill_markdown_with_hooks(
        config,
        app_data_dir,
        request,
        |skill, relative_path| fetch_remote_skill_md(skill, relative_path),
        |base_url, group, skill_id| {
            skill_hub_client::download_archive(base_url, group, skill_id)
        },
    )
}

pub(crate) fn read_skill_markdown_with_hooks<F, G>(
    config: &AppConfig,
    app_data_dir: &Path,
    request: SkillMarkdownRequestDto,
    fetch_remote_file: F,
    download_hub_archive: G,
) -> Result<SkillMarkdownPreviewDto, AppError>
where
    F: FnOnce(&DiscoverableSkill, &str) -> Result<String, AppError>,
    G: FnOnce(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    match request {
        SkillMarkdownRequestDto::Installed { storage_key } => {
            read_installed_skill_markdown(config.settings.main_skills_dir.as_deref(), &storage_key)
        }
        SkillMarkdownRequestDto::Discover { discover_key } => {
            let skill = config
                .skill_discover_cache
                .skills
                .iter()
                .find(|skill| skill.key == discover_key)
                .cloned()
                .ok_or_else(|| AppError::Io {
                    path: None,
                    message: format!("未找到可发现 skill：{discover_key}"),
                })?;
            read_discover_skill_markdown(
                config,
                app_data_dir,
                &skill,
                fetch_remote_file,
                download_hub_archive,
            )
        }
    }
}

fn read_discover_skill_markdown<F, G>(
    config: &AppConfig,
    app_data_dir: &Path,
    skill: &DiscoverableSkill,
    fetch_remote_file: F,
    download_hub_archive: G,
) -> Result<SkillMarkdownPreviewDto, AppError>
where
    F: FnOnce(&DiscoverableSkill, &str) -> Result<String, AppError>,
    G: FnOnce(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    if skill.source == "skillhub" {
        return read_hub_discover_skill_markdown(config, skill, download_hub_archive);
    }

    read_git_discover_skill_markdown(app_data_dir, skill, fetch_remote_file)
}

fn read_hub_discover_skill_markdown<G>(
    config: &AppConfig,
    skill: &DiscoverableSkill,
    download_hub_archive: G,
) -> Result<SkillMarkdownPreviewDto, AppError>
where
    G: FnOnce(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let base_url = skill_hub_endpoints::hub_endpoint_base_url(config, &skill.hub_endpoint_id)?;
    let zip_path = download_hub_archive(
        &base_url,
        &skill.hub_skill_group,
        &skill.hub_skill_id,
    )?;
    let raw = match extract_skill_md_from_zip(&zip_path) {
        Ok(raw) => {
            let _ = fs::remove_file(&zip_path);
            raw
        }
        Err(err) => {
            let _ = fs::remove_file(&zip_path);
            return Err(err);
        }
    };
    Ok(preview_from_raw(
        &raw,
        &skill.name,
        Some(skill.description.clone()),
        "hubArchive",
    ))
}

fn read_git_discover_skill_markdown<F>(
    app_data_dir: &Path,
    skill: &DiscoverableSkill,
    fetch_remote_file: F,
) -> Result<SkillMarkdownPreviewDto, AppError>
where
    F: FnOnce(&DiscoverableSkill, &str) -> Result<String, AppError>,
{
    let tree_dir = repo_tree_dir(app_data_dir, skill);
    for candidate in candidate_skill_md_paths(&tree_dir, skill) {
        if candidate.is_file() {
            let raw = fs::read_to_string(&candidate).map_err(|err| AppError::Io {
                path: Some(candidate.clone()),
                message: err.to_string(),
            })?;
            return Ok(preview_from_raw(
                &raw,
                &skill.name,
                Some(skill.description.clone()),
                "repoCache",
            ));
        }
    }

    let relative_path = skill_md_relative_path(skill);
    let raw = fetch_remote_file(skill, &relative_path)?;
    Ok(preview_from_raw(
        &raw,
        &skill.name,
        Some(skill.description.clone()),
        "remoteFile",
    ))
}

fn repo_tree_dir(app_data_dir: &Path, skill: &DiscoverableSkill) -> PathBuf {
    let host = if skill.repo_host.is_empty() {
        "github.com".to_string()
    } else {
        skill.repo_host.clone()
    };
    let project_path = if skill.project_path.is_empty() {
        format!("{}/{}", skill.repo_owner, skill.repo_name)
    } else {
        skill.project_path.clone()
    };
    let slug = if skill.repo_slug.is_empty() {
        skill_storage::compute_repo_slug(&host, &project_path)
    } else {
        skill.repo_slug.clone()
    };
    repo_cache::cache_root(app_data_dir)
        .join(slug)
        .join(&skill.repo_branch)
        .join("tree")
}

fn candidate_skill_md_paths(tree_dir: &Path, skill: &DiscoverableSkill) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if !skill.directory.is_empty() {
        paths.push(
            skill_install::resolve_skill_directory(tree_dir, &skill.directory).join("SKILL.md"),
        );
    } else {
        paths.push(tree_dir.join("SKILL.md"));
    }

    if !skill.install_dir_name.is_empty() {
        let alt = tree_dir.join(&skill.install_dir_name).join("SKILL.md");
        if !paths.iter().any(|path| path == &alt) {
            paths.push(alt);
        }
    }

    paths
}

fn skill_md_relative_path(skill: &DiscoverableSkill) -> String {
    if skill.directory.is_empty() {
        "SKILL.md".to_string()
    } else {
        format!(
            "{}/SKILL.md",
            skill.directory.trim_matches(|ch| ch == '/' || ch == '\\')
        )
    }
}

fn fetch_remote_skill_md(skill: &DiscoverableSkill, relative_path: &str) -> Result<String, AppError> {
    let repo_ref = skill.to_repo_ref();
    if repo_ref.provider == "gitlab" {
        let token = credential_store::get_gitlab_token(&repo_ref.host)?;
        return gitlab_client::fetch_file_raw(
            &repo_ref.host,
            &repo_ref.project_path,
            relative_path,
            &repo_ref.branch,
            token.as_deref(),
        );
    }

    fetch_github_raw_file(
        &skill.repo_owner,
        &skill.repo_name,
        &repo_ref.branch,
        relative_path,
    )
}

fn fetch_github_raw_file(
    owner: &str,
    name: &str,
    branch: &str,
    relative_path: &str,
) -> Result<String, AppError> {
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        owner, name, branch, relative_path
    );
    let response = reqwest::blocking::Client::new()
        .get(&url)
        .send()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("读取 GitHub 文件失败: {err}"),
        })?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(AppError::DownloadFailed {
            url,
            status: Some(status),
            message: format!("读取 GitHub 文件失败，HTTP 状态码 {status}"),
        });
    }
    response.text().map_err(|err| AppError::Io {
        path: None,
        message: format!("读取 GitHub 文件内容失败: {err}"),
    })
}

fn extract_skill_md_from_zip(zip_path: &Path) -> Result<String, AppError> {
    let extract_dir = create_temp_extract_dir()?;
    let cleanup = |dir: &Path| {
        let _ = fs::remove_dir_all(dir);
    };

    if let Err(err) = skill_downloader::extract_zip_file(zip_path, &extract_dir) {
        cleanup(&extract_dir);
        return Err(err);
    }

    let root_skill = extract_dir.join("SKILL.md");
    let read_result = if root_skill.is_file() {
        fs::read_to_string(&root_skill).map_err(|err| AppError::Io {
            path: Some(root_skill.clone()),
            message: err.to_string(),
        })
    } else if let Some(nested) = find_skill_md_under(&extract_dir) {
        fs::read_to_string(&nested).map_err(|err| AppError::Io {
            path: Some(nested),
            message: err.to_string(),
        })
    } else {
        Err(AppError::SkillDirNotFound {
            path: root_skill,
        })
    };

    cleanup(&extract_dir);
    read_result
}

fn create_temp_extract_dir() -> Result<PathBuf, AppError> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("无法创建临时目录: {err}"),
        })?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skills-sync-skill-md-{nanos}"));
    fs::create_dir_all(&dir).map_err(|err| AppError::Io {
        path: Some(dir.clone()),
        message: err.to_string(),
    })?;
    Ok(dir)
}

fn find_skill_md_under(root: &Path) -> Option<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
            {
                return Some(path);
            }
        }
    }
    None
}

fn preview_from_raw(
    raw: &str,
    fallback_title: &str,
    fallback_description: Option<String>,
    origin: &str,
) -> SkillMarkdownPreviewDto {
    let parts = parse_skill_markdown_preview(raw);
    SkillMarkdownPreviewDto {
        title: parts
            .title
            .unwrap_or_else(|| fallback_title.to_string()),
        description: parts
            .description
            .or(fallback_description)
            .unwrap_or_default(),
        markdown_body: parts.markdown_body,
        origin: origin.to_string(),
    }
}

fn trim_non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SkillDiscoverCache, SkillHubEndpoint};
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn parse_strips_frontmatter_and_keeps_body() {
        let raw = "---\nname: Demo\ndescription: Hello\n---\n\n# Title\n\nBody\n";
        let parts = parse_skill_markdown_preview(raw);
        assert_eq!(parts.title.as_deref(), Some("Demo"));
        assert_eq!(parts.description.as_deref(), Some("Hello"));
        assert!(parts.markdown_body.contains("# Title"));
        assert!(!parts.markdown_body.contains("description:"));
    }

    #[test]
    fn read_installed_returns_main_library_origin() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_dir = temp.path().join("demo-skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: Demo\ndescription: Hello\n---\n\n# Title\n\nBody\n",
        )
        .expect("write skill md");

        let preview = read_installed_skill_markdown(Some(temp.path()), "demo-skill")
            .expect("read installed");

        assert_eq!(preview.origin, "mainLibrary");
        assert_eq!(preview.title, "Demo");
        assert_eq!(preview.description, "Hello");
        assert!(preview.markdown_body.contains("# Title"));
        assert!(!preview.markdown_body.contains("description:"));
    }

    fn sample_git_skill() -> DiscoverableSkill {
        let repo_slug = skill_storage::compute_repo_slug("github.com", "anthropics/skills");
        DiscoverableSkill {
            key: "github.com/anthropics/skills:skills/demo".to_string(),
            name: "Demo".to_string(),
            description: "Cached skill.".to_string(),
            directory: "skills/demo".to_string(),
            install_dir_name: "demo".to_string(),
            repo_host: "github.com".to_string(),
            project_path: "anthropics/skills".to_string(),
            repo_owner: "anthropics".to_string(),
            repo_name: "skills".to_string(),
            repo_branch: "main".to_string(),
            source: "github".to_string(),
            storage_key: skill_storage::storage_key_for_repo(&repo_slug, "demo"),
            link_name: "demo".to_string(),
            repo_slug,
            ..Default::default()
        }
    }

    fn sample_hub_skill() -> DiscoverableSkill {
        DiscoverableSkill {
            key: "company-hub:common/tdd".to_string(),
            name: "tdd".to_string(),
            description: "Hub skill.".to_string(),
            directory: "common/tdd".to_string(),
            install_dir_name: "tdd".to_string(),
            source: "skillhub".to_string(),
            storage_key: skill_storage::storage_key_for_hub("company-hub", "common", "tdd"),
            link_name: "tdd".to_string(),
            hub_endpoint_id: "company-hub".to_string(),
            hub_skill_group: "common".to_string(),
            hub_skill_id: "tdd".to_string(),
            ..Default::default()
        }
    }

    fn create_hub_skill_zip(dir: &Path, skill_name: &str) -> PathBuf {
        let zip_path = dir.join("hub-skill.zip");
        let file = fs::File::create(&zip_path).expect("create zip");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let content = format!(
            "---\nname: {}\ndescription: From archive.\n---\n\n# Archive Body\n",
            skill_name
        );
        writer
            .start_file("SKILL.md", options)
            .expect("start file");
        writer
            .write_all(content.as_bytes())
            .expect("write skill md");
        writer.finish().expect("finish zip");
        zip_path
    }

    #[test]
    fn discover_git_uses_repo_cache_without_remote_fetch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let skill = sample_git_skill();
        let skill_md = repo_tree_dir(&app_data, &skill)
            .join("skills")
            .join("demo")
            .join("SKILL.md");
        fs::create_dir_all(skill_md.parent().expect("parent")).expect("mkdir");
        fs::write(
            &skill_md,
            "---\nname: Cached\ndescription: From cache.\n---\n\n# Cache Body\n",
        )
        .expect("write skill md");

        let remote_calls = AtomicUsize::new(0);
        let config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-07-15T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            ..Default::default()
        };

        let preview = read_skill_markdown_with_hooks(
            &config,
            &app_data,
            SkillMarkdownRequestDto::Discover {
                discover_key: skill.key.clone(),
            },
            |_skill, _path| {
                remote_calls.fetch_add(1, Ordering::SeqCst);
                Err(AppError::Io {
                    path: None,
                    message: "should not fetch".into(),
                })
            },
            |_, _, _| {
                Err(AppError::Io {
                    path: None,
                    message: "should not download hub".into(),
                })
            },
        )
        .expect("read from cache");

        assert_eq!(preview.origin, "repoCache");
        assert_eq!(preview.title, "Cached");
        assert!(preview.markdown_body.contains("# Cache Body"));
        assert_eq!(remote_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn discover_git_fetches_remote_file_when_cache_misses() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let skill = sample_git_skill();
        let remote_calls = AtomicUsize::new(0);
        let config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-07-15T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            ..Default::default()
        };

        let preview = read_skill_markdown_with_hooks(
            &config,
            &app_data,
            SkillMarkdownRequestDto::Discover {
                discover_key: skill.key.clone(),
            },
            |_skill, relative_path| {
                remote_calls.fetch_add(1, Ordering::SeqCst);
                assert_eq!(relative_path, "skills/demo/SKILL.md");
                Ok("---\nname: Remote\ndescription: From remote.\n---\n\n# Remote Body\n".into())
            },
            |_, _, _| {
                Err(AppError::Io {
                    path: None,
                    message: "should not download hub".into(),
                })
            },
        )
        .expect("read remote file");

        assert_eq!(preview.origin, "remoteFile");
        assert_eq!(preview.title, "Remote");
        assert!(preview.markdown_body.contains("# Remote Body"));
        assert_eq!(remote_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn discover_hub_extracts_skill_md_from_archive() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path().join("app-data");
        let skill = sample_hub_skill();
        let zip_path = create_hub_skill_zip(temp.path(), "tdd");
        let config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-07-15T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            skill_hub_endpoints: vec![SkillHubEndpoint {
                id: "company-hub".to_string(),
                name: "Company Hub".to_string(),
                base_url: "https://hub.example.com".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };

        let preview = read_skill_markdown_with_hooks(
            &config,
            &app_data,
            SkillMarkdownRequestDto::Discover {
                discover_key: skill.key.clone(),
            },
            |_skill, _path| {
                Err(AppError::Io {
                    path: None,
                    message: "should not fetch git".into(),
                })
            },
            |base_url, group, skill_id| {
                assert_eq!(base_url, "https://hub.example.com");
                assert_eq!(group, "common");
                assert_eq!(skill_id, "tdd");
                Ok(zip_path.clone())
            },
        )
        .expect("read hub archive");

        assert_eq!(preview.origin, "hubArchive");
        assert_eq!(preview.title, "tdd");
        assert_eq!(preview.description, "From archive.");
        assert!(preview.markdown_body.contains("# Archive Body"));
    }

    #[test]
    fn discover_unknown_key_returns_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config = AppConfig::default();
        let err = read_skill_markdown_with_hooks(
            &config,
            temp.path(),
            SkillMarkdownRequestDto::Discover {
                discover_key: "missing:skill".into(),
            },
            |_, _| Ok(String::new()),
            |_, _, _| {
                Err(AppError::Io {
                    path: None,
                    message: "unused".into(),
                })
            },
        )
        .expect_err("unknown key");

        match err {
            AppError::Io { message, .. } => {
                assert!(message.contains("未找到可发现 skill：missing:skill"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn request_dto_deserializes_camel_case_fields() {
        let installed: SkillMarkdownRequestDto = serde_json::from_str(
            r#"{"kind":"installed","storageKey":"demo/skill"}"#,
        )
        .expect("installed request");
        assert_eq!(
            installed,
            SkillMarkdownRequestDto::Installed {
                storage_key: "demo/skill".into(),
            }
        );

        let discover: SkillMarkdownRequestDto = serde_json::from_str(
            r#"{"kind":"discover","discoverKey":"git:demo"}"#,
        )
        .expect("discover request");
        assert_eq!(
            discover,
            SkillMarkdownRequestDto::Discover {
                discover_key: "git:demo".into(),
            }
        );
    }
}
