use crate::models::{AppConfig, AppError, DiscoverableSkill, RepoRef, SkillRecord, default_github_host};
use crate::skill_discover::iso8601_timestamp_now;
use crate::skill_downloader::{self, copy_dir_recursive};
use crate::skill_updates;
use std::path::{Path, PathBuf};

pub fn install_to_main(
    config: &mut AppConfig,
    skill: &DiscoverableSkill,
    main_dir: &Path,
) -> Result<(), AppError> {
    install_to_main_with_download(config, skill, main_dir, |repo_ref| {
        skill_downloader::download_repo_ref(repo_ref)
    })
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
    let install_path = main_dir.join(&skill.install_dir_name);
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

    copy_dir_recursive(&source_dir, &install_path)?;

    let content_hash = skill_updates::compute_dir_hash(&install_path)?;

    config.skill_records.insert(
        skill.install_dir_name.clone(),
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

        let installed = main_dir.join("brainstorming");
        assert!(installed.join("SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(installed.join("SKILL.md")).unwrap(),
            fs::read_to_string(repo_root.join("skills").join("brainstorming").join("SKILL.md")).unwrap()
        );

        let record = config
            .skill_records
            .get("brainstorming")
            .expect("skill record");
        assert_eq!(record.repo_owner, "anthropics");
        assert_eq!(record.source, "github");
        assert_eq!(record.directory, "skills/brainstorming");
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
        fs::create_dir_all(main_dir.join("brainstorming")).expect("create existing dir");
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
        assert!(!main_dir.join("brainstorming").exists());
        assert!(config.skill_records.is_empty());
    }

    #[test]
    fn resolve_skill_directory_joins_nested_segments() {
        let root = PathBuf::from("/repo");
        let resolved = resolve_skill_directory(&root, "skills/brainstorming");
        assert_eq!(resolved, PathBuf::from("/repo/skills/brainstorming"));
    }

}
