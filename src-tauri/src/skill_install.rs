use crate::models::{AppConfig, AppError, DiscoverableSkill, RepoRef, SkillRecord, default_github_host};
use crate::skill_discover::iso8601_timestamp_now;
use crate::skill_downloader::{self, copy_dir_recursive};
use crate::iflytek_skill_hub_client;
use crate::iflytek_skill_hub_endpoints;
use crate::skill_hub_client;
use crate::skill_hub_endpoints;
use crate::skill_storage;
use crate::skill_updates;
use std::fs;
use std::path::{Path, PathBuf};

fn resolve_install_path(main_dir: &Path, skill: &DiscoverableSkill) -> PathBuf {
    if !skill.storage_key.is_empty() {
        skill_storage::main_library_path(main_dir, &skill.storage_key)
    } else {
        main_dir.join(&skill.install_dir_name)
    }
}

fn resolve_record_key(skill: &DiscoverableSkill) -> String {
    if !skill.storage_key.is_empty() {
        skill.storage_key.clone()
    } else {
        skill.install_dir_name.clone()
    }
}

fn resolve_record_storage_fields(skill: &DiscoverableSkill) -> (String, String, String) {
    let link_name = if !skill.link_name.is_empty() {
        skill.link_name.clone()
    } else {
        skill_storage::skill_id_from_directory(&skill.directory)
    };

    let repo_host = if skill.repo_host.is_empty() {
        default_github_host()
    } else {
        skill.repo_host.clone()
    };

    let project_path = if skill.project_path.is_empty() {
        format!("{}/{}", skill.repo_owner, skill.repo_name)
    } else {
        skill.project_path.clone()
    };

    let repo_slug = if !skill.repo_slug.is_empty() {
        skill.repo_slug.clone()
    } else if matches!(skill.source.as_str(), "github" | "gitlab" | "skillssh") {
        skill_storage::compute_repo_slug(&repo_host, &project_path)
    } else {
        String::new()
    };

    let storage_key = if !skill.storage_key.is_empty() {
        skill.storage_key.clone()
    } else {
        skill_storage::storage_key_from_record_source(
            &skill.source,
            if repo_slug.is_empty() {
                None
            } else {
                Some(&repo_slug)
            },
            if skill.hub_endpoint_id.is_empty() {
                None
            } else {
                Some(&skill.hub_endpoint_id)
            },
            if skill.hub_skill_group.is_empty() {
                None
            } else {
                Some(&skill.hub_skill_group)
            },
            &link_name,
        )
    };

    (storage_key, link_name, repo_slug)
}

fn purge_discover_cache_entry(config: &mut AppConfig, skill: &DiscoverableSkill) {
    config
        .skill_discover_cache
        .skills
        .retain(|cached| cached.key != skill.key);
}

pub fn install_to_main(
    config: &mut AppConfig,
    skill: &DiscoverableSkill,
    main_dir: &Path,
) -> Result<(), AppError> {
    if skill.source == "skillhub" {
        return install_hub_to_main_with_download(config, skill, main_dir, |base_url, group, skill_id| {
            skill_hub_client::download_archive(base_url, group, skill_id)
        });
    }
    if skill.source == "iflytek" {
        return install_hub_to_main_with_download(config, skill, main_dir, |base_url, group, skill_id| {
            iflytek_skill_hub_client::download_skill_zip(base_url, group, skill_id)
        });
    }

    install_to_main_with_download(config, skill, main_dir, |repo_ref| {
        skill_downloader::download_repo_ref(repo_ref)
    })
}

fn hub_install_base_url(config: &AppConfig, skill: &DiscoverableSkill) -> Result<String, AppError> {
    if skill.source == "iflytek" {
        iflytek_skill_hub_endpoints::iflytek_endpoint_base_url(config, &skill.hub_endpoint_id)
    } else {
        skill_hub_endpoints::hub_endpoint_base_url(config, &skill.hub_endpoint_id)
    }
}

pub fn install_hub_to_main_with_download<F>(
    config: &mut AppConfig,
    skill: &DiscoverableSkill,
    main_dir: &Path,
    download_archive: F,
) -> Result<(), AppError>
where
    F: FnOnce(&str, &str, &str) -> Result<PathBuf, AppError>,
{
    let install_path = resolve_install_path(main_dir, skill);
    if install_path.exists() {
        return Err(AppError::DirExists {
            path: install_path,
        });
    }

    let base_url = hub_install_base_url(config, skill)?;
    let zip_path = match download_archive(
        &base_url,
        &skill.hub_skill_group,
        &skill.hub_skill_id,
    ) {
        Ok(path) => path,
        Err(err) => {
            if matches!(&err, AppError::HubSkillGone { .. }) {
                purge_discover_cache_entry(config, skill);
            }
            return Err(err);
        }
    };

    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent).map_err(|err| AppError::Io {
            path: Some(parent.to_path_buf()),
            message: err.to_string(),
        })?;
    }
    fs::create_dir_all(&install_path).map_err(|err| AppError::Io {
        path: Some(install_path.clone()),
        message: err.to_string(),
    })?;

    let extract_result = skill_downloader::extract_zip_file(&zip_path, &install_path);
    let _ = fs::remove_file(&zip_path);
    extract_result?;

    if !install_path.join("SKILL.md").is_file() {
        let _ = fs::remove_dir_all(&install_path);
        return Err(AppError::SkillDirNotFound {
            path: install_path.join("SKILL.md"),
        });
    }

    let content_hash = skill_updates::compute_dir_hash(&install_path)?;
    let (storage_key, link_name, repo_slug) = resolve_record_storage_fields(skill);

    config.skill_records.insert(
        resolve_record_key(skill),
        SkillRecord {
            repo_host: String::new(),
            project_path: String::new(),
            source: skill.source.clone(),
            repo_owner: String::new(),
            repo_name: String::new(),
            repo_branch: String::new(),
            directory: skill.directory.clone(),
            content_hash,
            installed_at: iso8601_timestamp_now(),
            storage_key,
            link_name,
            repo_slug,
            hub_endpoint_id: skill.hub_endpoint_id.clone(),
            hub_skill_group: skill.hub_skill_group.clone(),
            hub_skill_id: skill.hub_skill_id.clone(),
            source_missing: false,
        },
    );

    config
        .skill_discover_cache
        .skills
        .retain(|cached| cached.key != skill.key);

    Ok(())
}

pub fn install_to_main_with_download<F>(
    config: &mut AppConfig,
    skill: &DiscoverableSkill,
    main_dir: &Path,
    download_repo_ref: F,
) -> Result<(), AppError>
where
    F: FnOnce(&RepoRef) -> Result<PathBuf, AppError>,
{
    let install_path = resolve_install_path(main_dir, skill);
    if install_path.exists() {
        return Err(AppError::DirExists {
            path: install_path,
        });
    }

    let repo_ref = skill.to_repo_ref();
    let repo_root = download_repo_ref(&repo_ref)?;
    let source_dir = resolve_skill_directory(&repo_root, &skill.directory);

    if !source_dir.join("SKILL.md").is_file() {
        return Err(AppError::SkillDirNotFound {
            path: source_dir,
        });
    }

    if let Some(parent) = install_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| AppError::Io {
            path: Some(parent.to_path_buf()),
            message: err.to_string(),
        })?;
    }

    copy_dir_recursive(&source_dir, &install_path)?;

    let content_hash = skill_updates::compute_dir_hash(&install_path)?;

    let (storage_key, link_name, repo_slug) = resolve_record_storage_fields(skill);

    config.skill_records.insert(
        resolve_record_key(skill),
        SkillRecord {
            repo_host: if skill.repo_host.is_empty() {
                default_github_host()
            } else {
                skill.repo_host.clone()
            },
            project_path: if skill.project_path.is_empty() {
                format!("{}/{}", skill.repo_owner, skill.repo_name)
            } else {
                skill.project_path.clone()
            },
            source: skill.source.clone(),
            repo_owner: skill.repo_owner.clone(),
            repo_name: skill.repo_name.clone(),
            repo_branch: skill.repo_branch.clone(),
            directory: skill.directory.clone(),
            content_hash,
            installed_at: iso8601_timestamp_now(),
            storage_key,
            link_name,
            repo_slug,
            hub_endpoint_id: skill.hub_endpoint_id.clone(),
            hub_skill_group: skill.hub_skill_group.clone(),
            hub_skill_id: skill.hub_skill_id.clone(),
            source_missing: false,
        },
    );

    config
        .skill_discover_cache
        .skills
        .retain(|cached| cached.key != skill.key);

    Ok(())
}

pub fn resolve_skill_directory(repo_root: &Path, directory: &str) -> PathBuf {
    let mut path = repo_root.to_path_buf();
    for segment in directory.split('/').filter(|segment| !segment.is_empty()) {
        path.push(segment);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SkillDiscoverCache;
    use std::fs;

    fn write_valid_skill(dir: &Path, name: &str) {
        fs::create_dir_all(dir).expect("create skill dir");
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
                name
            ),
        )
        .expect("write skill md");
    }

    fn sample_skill(key_suffix: &str) -> DiscoverableSkill {
        let repo_slug = crate::skill_storage::compute_repo_slug(
            &default_github_host(),
            "anthropics/skills",
        );
        let storage_key =
            crate::skill_storage::storage_key_for_repo(&repo_slug, key_suffix);
        DiscoverableSkill {
            key: format!("github.com/anthropics/skills:skills/{key_suffix}"),
            name: key_suffix.to_string(),
            description: "Test skill.".to_string(),
            directory: format!("skills/{key_suffix}"),
            install_dir_name: key_suffix.to_string(),
            repo_host: default_github_host(),
            project_path: "anthropics/skills".to_string(),
            repo_owner: "anthropics".to_string(),
            repo_name: "skills".to_string(),
            repo_branch: "main".to_string(),
            source: "github".to_string(),
            storage_key,
            link_name: key_suffix.to_string(),
            repo_slug,
            ..Default::default()
        }
    }

    fn install_with_local_repo(
        config: &mut AppConfig,
        skill: &DiscoverableSkill,
        main_dir: &Path,
        repo_root: &Path,
    ) -> Result<(), AppError> {
        install_to_main_with_download(config, skill, main_dir, |_| Ok(repo_root.to_path_buf()))
    }

    #[test]
    fn install_to_main_copies_skill_and_updates_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");
        write_valid_skill(&repo_root.join("skills").join("brainstorming"), "brainstorming");

        let skill = sample_skill("brainstorming");
        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-06-30T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            ..Default::default()
        };

        install_with_local_repo(&mut config, &skill, &main_dir, &repo_root).expect("install");

        let installed = main_dir
            .join("repo")
            .join("github.com--anthropics-skills")
            .join("brainstorming");
        assert!(installed.join("SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(installed.join("SKILL.md")).unwrap(),
            fs::read_to_string(repo_root.join("skills").join("brainstorming").join("SKILL.md")).unwrap()
        );

        let record = config
            .skill_records
            .get("repo/github.com--anthropics-skills/brainstorming")
            .expect("skill record");
        assert_eq!(record.repo_owner, "anthropics");
        assert_eq!(record.source, "github");
        assert_eq!(record.directory, "skills/brainstorming");
        assert_eq!(record.storage_key, "repo/github.com--anthropics-skills/brainstorming");
        assert_eq!(record.link_name, "brainstorming");
        assert_eq!(record.repo_slug, "github.com--anthropics-skills");
        assert!(!record.content_hash.is_empty());
        assert!(!record.installed_at.is_empty());
        assert!(config.skill_discover_cache.skills.is_empty());
    }

    #[test]
    fn install_to_main_returns_dir_exists_when_target_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");
        fs::create_dir_all(
            main_dir
                .join("repo")
                .join("github.com--anthropics-skills")
                .join("brainstorming"),
        )
        .expect("create existing dir");
        write_valid_skill(&repo_root.join("skills").join("brainstorming"), "brainstorming");

        let skill = sample_skill("brainstorming");
        let mut config = AppConfig::default();

        let error = install_with_local_repo(&mut config, &skill, &main_dir, &repo_root)
            .expect_err("should fail when dir exists");

        assert!(matches!(error, AppError::DirExists { .. }));
        assert!(config.skill_records.is_empty());
    }

    #[test]
    fn install_to_main_returns_skill_dir_not_found_without_skill_md() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");
        fs::create_dir_all(repo_root.join("skills").join("brainstorming")).expect("create source dir");

        let skill = sample_skill("brainstorming");
        let mut config = AppConfig::default();

        let error = install_with_local_repo(&mut config, &skill, &main_dir, &repo_root)
            .expect_err("should fail without SKILL.md");

        assert!(matches!(error, AppError::SkillDirNotFound { .. }));
        assert!(
            !main_dir
                .join("repo")
                .join("github.com--anthropics-skills")
                .join("brainstorming")
                .exists()
        );
        assert!(config.skill_records.is_empty());
    }

    #[test]
    fn resolve_skill_directory_joins_nested_segments() {
        let root = PathBuf::from("/repo");
        let resolved = resolve_skill_directory(&root, "skills/brainstorming");
        assert_eq!(resolved, PathBuf::from("/repo/skills/brainstorming"));
    }

    fn sample_hub_skill(skill_id: &str) -> DiscoverableSkill {
        let storage_key =
            skill_storage::storage_key_for_hub("company-hub", "common", skill_id);
        DiscoverableSkill {
            key: format!("company-hub:common/{skill_id}"),
            name: skill_id.to_string(),
            description: "Hub skill.".to_string(),
            directory: format!("common/{skill_id}"),
            install_dir_name: skill_id.to_string(),
            source: "skillhub".to_string(),
            storage_key,
            link_name: skill_id.to_string(),
            hub_endpoint_id: "company-hub".to_string(),
            hub_skill_group: "common".to_string(),
            hub_skill_id: skill_id.to_string(),
            ..Default::default()
        }
    }

    fn sample_iflytek_skill(skill_id: &str) -> DiscoverableSkill {
        let storage_key = skill_storage::storage_key_for_hub("xkw", "global", skill_id);
        DiscoverableSkill {
            key: format!("xkw:global/{skill_id}"),
            name: skill_id.to_string(),
            description: "iFlytek skill.".to_string(),
            directory: format!("global/{skill_id}"),
            install_dir_name: skill_id.to_string(),
            source: "iflytek".to_string(),
            storage_key,
            link_name: skill_id.to_string(),
            hub_endpoint_id: "xkw".to_string(),
            hub_skill_group: "global".to_string(),
            hub_skill_id: skill_id.to_string(),
            ..Default::default()
        }
    }

    fn create_hub_skill_zip(dir: &Path, skill_name: &str) -> PathBuf {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let zip_path = dir.join("hub-skill.zip");
        let file = fs::File::create(&zip_path).expect("create zip");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let content = format!(
            "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
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
    fn install_hub_to_main_extracts_archive_and_updates_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let zip_path = create_hub_skill_zip(temp.path(), "tdd");
        let skill = sample_hub_skill("tdd");
        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-06-30T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            skill_hub_endpoints: vec![crate::models::SkillHubEndpoint {
                id: "company-hub".to_string(),
                name: "Company Hub".to_string(),
                base_url: "https://hub.example.com".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };

        install_hub_to_main_with_download(&mut config, &skill, &main_dir, |_, _, _| {
            Ok(zip_path.clone())
        })
        .expect("install hub skill");

        let installed = main_dir
            .join("hub")
            .join("company-hub")
            .join("common")
            .join("tdd");
        assert!(installed.join("SKILL.md").is_file());

        let record = config
            .skill_records
            .get("hub/company-hub/common/tdd")
            .expect("skill record");
        assert_eq!(record.source, "skillhub");
        assert_eq!(record.hub_endpoint_id, "company-hub");
        assert_eq!(record.hub_skill_group, "common");
        assert_eq!(record.hub_skill_id, "tdd");
        assert_eq!(record.storage_key, "hub/company-hub/common/tdd");
        assert_eq!(record.link_name, "tdd");
        assert!(!record.content_hash.is_empty());
        assert!(config.skill_discover_cache.skills.is_empty());
    }

    #[test]
    fn install_hub_gone_purges_discover_cache_without_installing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let skill = DiscoverableSkill {
            key: "hub:company-hub:tools:brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "gone".to_string(),
            directory: "tools/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            source: "skillhub".to_string(),
            storage_key: "hub/company-hub/tools/brainstorming".to_string(),
            link_name: "brainstorming".to_string(),
            hub_endpoint_id: "company-hub".to_string(),
            hub_skill_group: "tools".to_string(),
            hub_skill_id: "brainstorming".to_string(),
            ..Default::default()
        };

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-06-30T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            skill_hub_endpoints: vec![crate::models::SkillHubEndpoint {
                id: "company-hub".to_string(),
                name: "Company Hub".to_string(),
                base_url: "https://hub.example.com".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };

        let err = install_hub_to_main_with_download(&mut config, &skill, &main_dir, |_, _, _| {
            Err(AppError::HubSkillGone {
                skill_id: "brainstorming".to_string(),
                group: "tools".to_string(),
            })
        })
        .expect_err("should fail when hub skill gone");

        assert!(matches!(err, AppError::HubSkillGone { .. }));
        assert!(config.skill_discover_cache.skills.is_empty());
        assert!(config.skill_records.is_empty());
        assert!(!main_dir
            .join("hub")
            .join("company-hub")
            .join("tools")
            .join("brainstorming")
            .exists());
    }

    #[test]
    fn install_iflytek_uses_namespace_slug_download_and_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let zip_path = create_hub_skill_zip(temp.path(), "x");
        let skill = sample_iflytek_skill("x");
        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: Some("2026-06-30T00:00:00Z".to_string()),
                skills: vec![skill.clone()],
            },
            iflytek_skill_hub_endpoints: vec![crate::models::IflytekSkillHubEndpoint {
                id: "xkw".to_string(),
                name: "XKW Hub".to_string(),
                base_url: "https://iflytek.example.com".to_string(),
                enabled: true,
            }],
            ..Default::default()
        };

        install_hub_to_main_with_download(&mut config, &skill, &main_dir, |base_url, namespace, slug| {
            assert_eq!(base_url, "https://iflytek.example.com");
            assert_eq!(namespace, "global");
            assert_eq!(slug, "x");
            Ok(zip_path.clone())
        })
        .expect("install iflytek skill");

        let installed = main_dir.join("hub").join("xkw").join("global").join("x");
        assert!(installed.join("SKILL.md").is_file());

        let record = config
            .skill_records
            .get("hub/xkw/global/x")
            .expect("skill record");
        assert_eq!(record.source, "iflytek");
        assert_eq!(record.hub_endpoint_id, "xkw");
        assert_eq!(record.hub_skill_group, "global");
        assert_eq!(record.hub_skill_id, "x");
        assert!(config.skill_discover_cache.skills.is_empty());
    }

}
