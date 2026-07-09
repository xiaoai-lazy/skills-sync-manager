use crate::models::{AppConfig, AppError, DiscoverableSkill, RepoRef, SkillDiscoverCache, SkillRecord, SkillRepo, default_github_provider};
use crate::repo_cache;
use crate::skill_library;
use crate::skill_storage;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// 扫描所有已启用来源；单个仓库下载失败时跳过并记录警告，不中断其余来源。
pub fn discover_available_with_warnings(
    config: &AppConfig,
    main_dir: Option<&Path>,
    app_data_dir: &Path,
    force: bool,
) -> (Vec<DiscoverableSkill>, Vec<String>) {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();

    for repo in config.skill_repos.iter().filter(|repo| repo.enabled) {
        let repo_ref = repo.to_repo_ref();
        let repo_label = repo_display_label(repo);
        match repo_cache::ensure_repo_tree(repo, app_data_dir, force) {
            Ok(repo_root) => {
                skills.extend(fetch_repo_skills_from_path(&repo_root, &repo_ref));
            }
            Err(err) => {
                warnings.push(format!("跳过来源 {}：{}", repo_label, err.to_dto().message));
            }
        }
    }

    let filtered = filter_uninstalled_discoverable_skills(
        skills,
        main_dir,
        Some(&config.skill_records),
    );

    (deduplicate_discoverable_skills(filtered), warnings)
}

fn repo_display_label(repo: &SkillRepo) -> String {
    if repo.project_path.is_empty() {
        format!("{}/{}/{}", repo.host, repo.owner, repo.name)
    } else {
        format!("{}/{}", repo.host, repo.project_path)
    }
}

/// 仅下载并扫描单个来源仓库中的 Skill。
pub fn discover_repo_skills(
    repo: &SkillRepo,
    main_dir: Option<&Path>,
    app_data_dir: &Path,
    force: bool,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    if !repo.enabled {
        return Ok(Vec::new());
    }

    let repo_ref = repo.to_repo_ref();
    let repo_root = repo_cache::ensure_repo_tree(repo, app_data_dir, force)?;
    let skills = fetch_repo_skills_from_path(&repo_root, &repo_ref);
    Ok(filter_uninstalled_discoverable_skills(
        skills,
        main_dir,
        None,
    ))
}

/// 将单个仓库的 discover 结果合并进缓存，不影响其他来源仓库的缓存条目。
pub fn merge_repo_into_discover_cache(
    config: &mut AppConfig,
    repo: &SkillRepo,
    main_dir: Option<&Path>,
    app_data_dir: &Path,
    force: bool,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    let new_repo_skills = discover_repo_skills(repo, main_dir, app_data_dir, force)?;
    let retained = config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| !skill_belongs_to_repo(skill, repo))
        .cloned()
        .collect::<Vec<_>>();
    let mut merged = retained;
    merged.extend(new_repo_skills);
    let skills = deduplicate_discoverable_skills(merged);

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    Ok(skills)
}

/// 从 discover 缓存中移除指定来源仓库的 Skill，不触发任何下载。
pub fn remove_repo_from_discover_cache(
    config: &mut AppConfig,
    host: &str,
    project_path: &str,
) -> Vec<DiscoverableSkill> {
    let skills = config
        .skill_discover_cache
        .skills
        .iter()
        .filter(|skill| !skill_belongs_to_host_and_path(skill, host, project_path))
        .cloned()
        .collect::<Vec<_>>();

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    skills
}

fn skill_belongs_to_repo(skill: &DiscoverableSkill, repo: &SkillRepo) -> bool {
    skill_belongs_to_host_and_path(skill, &repo.host, &repo.project_path)
}

fn skill_belongs_to_host_and_path(
    skill: &DiscoverableSkill,
    host: &str,
    project_path: &str,
) -> bool {
    let skill_path = if skill.project_path.is_empty() {
        format!("{}/{}", skill.repo_owner, skill.repo_name)
    } else {
        skill.project_path.clone()
    };
    skill.repo_host.eq_ignore_ascii_case(host.trim())
        && skill_path.eq_ignore_ascii_case(project_path.trim())
}

pub fn fetch_repo_skills_from_path(
    repo_root: &Path,
    repo_ref: &RepoRef,
) -> Vec<DiscoverableSkill> {
    let mut skills = Vec::new();
    scan_dir_recursive(repo_root, repo_root, repo_ref, &mut skills);
    skills
}

fn scan_dir_recursive(
    repo_root: &Path,
    current: &Path,
    repo_ref: &RepoRef,
    skills: &mut Vec<DiscoverableSkill>,
) {
    let skill_md = current.join("SKILL.md");
    if skill_md.is_file() {
        if let Ok(raw) = fs::read_to_string(&skill_md) {
            if let Some(metadata) = skill_library::parse_valid_skill_metadata(&raw) {
                let directory = relative_directory(repo_root, current);
                let install_dir_name = install_dir_name_from_directory(&directory);
                let key = format!(
                    "{}/{}:{}",
                    repo_ref.host, repo_ref.project_path, directory
                );
                let (repo_owner, repo_name) = project_path_to_owner_name(&repo_ref.project_path);
                let source = if repo_ref.provider == "gitlab" {
                    "gitlab".to_string()
                } else {
                    default_github_provider()
                };
                let skill_id = skill_storage::skill_id_from_directory(&directory);
                let repo_slug =
                    skill_storage::compute_repo_slug(&repo_ref.host, &repo_ref.project_path);
                let storage_key = skill_storage::storage_key_for_repo(&repo_slug, &skill_id);

                skills.push(DiscoverableSkill {
                    key,
                    name: metadata.name,
                    description: metadata.description,
                    directory,
                    install_dir_name,
                    repo_host: repo_ref.host.clone(),
                    project_path: repo_ref.project_path.clone(),
                    repo_owner,
                    repo_name,
                    repo_branch: repo_ref.branch.clone(),
                    source,
                    storage_key,
                    link_name: skill_id,
                    repo_slug,
                    hub_endpoint_id: String::new(),
                    hub_skill_group: String::new(),
                    hub_skill_id: String::new(),
                });
            }
        }
    }

    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(repo_root, &path, repo_ref, skills);
        }
    }
}

fn project_path_to_owner_name(project_path: &str) -> (String, String) {
    match project_path.rsplit_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (String::new(), project_path.to_string()),
    }
}

pub fn deduplicate_discoverable_skills(skills: Vec<DiscoverableSkill>) -> Vec<DiscoverableSkill> {
    let mut seen = HashSet::new();
    skills
        .into_iter()
        .filter(|skill| seen.insert(skill.key.clone()))
        .collect()
}

pub fn filter_uninstalled_discoverable_skills(
    skills: Vec<DiscoverableSkill>,
    main_dir: Option<&Path>,
    skill_records: Option<&HashMap<String, SkillRecord>>,
) -> Vec<DiscoverableSkill> {
    skills
        .into_iter()
        .filter(|skill| {
            !is_skill_installed(skill, main_dir) && !is_skill_in_records(skill, skill_records)
        })
        .collect()
}

fn skill_link_name(skill: &DiscoverableSkill) -> String {
    if !skill.link_name.is_empty() {
        skill.link_name.clone()
    } else {
        skill.install_dir_name.clone()
    }
}

fn is_skill_in_records(
    skill: &DiscoverableSkill,
    skill_records: Option<&HashMap<String, SkillRecord>>,
) -> bool {
    let Some(records) = skill_records else {
        return false;
    };

    if skill.source == "skillhub" {
        return is_hub_skill_in_records(skill, records);
    }

    if !skill.storage_key.is_empty() && records.contains_key(&skill.storage_key) {
        return true;
    }

    if !skill.storage_key.is_empty()
        && records
            .values()
            .any(|record| record.storage_key == skill.storage_key)
    {
        return true;
    }

    let link_name = skill_link_name(skill);
    if records.values().any(|record| record.link_name == link_name) {
        return true;
    }

    false
}

fn is_hub_skill_in_records(
    skill: &DiscoverableSkill,
    records: &HashMap<String, SkillRecord>,
) -> bool {
    if skill.hub_endpoint_id.is_empty() || skill.hub_skill_id.is_empty() {
        return false;
    }

    if !skill.storage_key.is_empty() {
        if records.contains_key(&skill.storage_key) {
            return true;
        }
        if records
            .values()
            .any(|record| record.storage_key == skill.storage_key)
        {
            return true;
        }
    }

    records.values().any(|record| {
        record.source == "skillhub"
            && record.hub_endpoint_id == skill.hub_endpoint_id
            && record.hub_skill_group == skill.hub_skill_group
            && record.hub_skill_id == skill.hub_skill_id
    })
}

fn is_skill_installed(skill: &DiscoverableSkill, main_dir: Option<&Path>) -> bool {
    let Some(main_dir) = main_dir else {
        return false;
    };

    if skill.source == "skillhub" {
        if skill.storage_key.is_empty() {
            return false;
        }
        return skill_storage::main_library_path(main_dir, &skill.storage_key).is_dir();
    }

    if main_dir.join(&skill.install_dir_name).is_dir() {
        return true;
    }

    if !skill.storage_key.is_empty() {
        let storage_path = skill_storage::main_library_path(main_dir, &skill.storage_key);
        if storage_path.is_dir() {
            return true;
        }
    }

    false
}

fn relative_directory(repo_root: &Path, skill_dir: &Path) -> String {
    skill_dir
        .strip_prefix(repo_root)
        .unwrap_or(skill_dir)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn install_dir_name_from_directory(directory: &str) -> String {
    directory
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(directory)
        .to_string()
}

pub fn iso8601_timestamp_now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppError, SkillRepo, default_github_host, default_github_provider};
    use std::fs;
    use std::path::PathBuf;

    fn write_valid_skill(dir: &Path, name: &str, description: &str) {
        fs::create_dir_all(dir).expect("create skill dir");
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: {}\n---\n\n# Skill\n",
                name, description
            ),
        )
        .expect("write skill md");
    }

    fn enabled_repo(owner: &str, name: &str) -> SkillRepo {
        SkillRepo {
            host: default_github_host(),
            provider: default_github_provider(),
            project_path: format!("{}/{}", owner, name),
            owner: owner.to_string(),
            name: name.to_string(),
            branch: "main".to_string(),
            enabled: true,
        }
    }

    fn github_repo_ref(owner: &str, name: &str) -> RepoRef {
        enabled_repo(owner, name).to_repo_ref()
    }

    #[test]
    fn discoverable_skill_has_storage_key_for_github() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_skill(
            &temp.path().join("skills").join("brainstorming"),
            "brainstorming",
            "Explore ideas.",
        );

        let repo_ref = github_repo_ref("anthropics", "skills");
        let skills = fetch_repo_skills_from_path(temp.path(), &repo_ref);

        assert_eq!(skills.len(), 1);
        let skill = &skills[0];
        assert_eq!(skill.directory, "skills/brainstorming");
        assert_eq!(skill.link_name, "brainstorming");
        assert_eq!(skill.repo_slug, "github.com--anthropics-skills");
        assert_eq!(
            skill.storage_key,
            "repo/github.com--anthropics-skills/brainstorming"
        );
    }

    #[test]
    fn fetch_repo_skills_from_path_discovers_nested_skills() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_skill(
            &temp.path().join("skills").join("brainstorming"),
            "brainstorming",
            "Explore ideas.",
        );
        write_valid_skill(
            &temp.path().join("skills").join("writing-plans"),
            "writing-plans",
            "Create implementation plans.",
        );

        let repo_ref = github_repo_ref("anthropics", "skills");
        let skills = fetch_repo_skills_from_path(temp.path(), &repo_ref);

        assert_eq!(skills.len(), 2);
        assert!(skills.iter().any(|skill| skill.install_dir_name == "brainstorming"));
        assert!(skills.iter().any(|skill| skill.install_dir_name == "writing-plans"));
        assert!(skills.iter().all(|skill| skill.source == "github"));
        assert!(skills
            .iter()
            .any(|skill| skill.key == "github.com/anthropics/skills:skills/brainstorming"));
    }

    #[test]
    fn fetch_repo_skills_from_path_sets_gitlab_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_skill(
            &temp.path().join("skills").join("my-skill"),
            "my-skill",
            "A GitLab skill.",
        );

        let repo_ref = RepoRef {
            host: "gitlab.example.com".to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/my-project".to_string(),
            branch: "main".to_string(),
        };
        let skills = fetch_repo_skills_from_path(temp.path(), &repo_ref);

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].source, "gitlab");
        assert_eq!(skills[0].repo_host, "gitlab.example.com");
        assert_eq!(skills[0].project_path, "group/my-project");
        assert_eq!(skills[0].repo_owner, "group");
        assert_eq!(skills[0].repo_name, "my-project");
        assert_eq!(
            skills[0].key,
            "gitlab.example.com/group/my-project:skills/my-skill"
        );
    }

    #[test]
    fn discover_available_excludes_skills_already_in_main_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        write_valid_skill(
            &repo_root.join("skills").join("brainstorming"),
            "brainstorming",
            "Explore ideas.",
        );
        write_valid_skill(
            &repo_root.join("skills").join("writing-plans"),
            "writing-plans",
            "Create implementation plans.",
        );
        write_valid_skill(&main_dir.join("brainstorming"), "brainstorming", "Installed copy.");

        let config = AppConfig {
            skill_repos: vec![enabled_repo("anthropics", "skills")],
            ..Default::default()
        };

        let skills = discover_available_from_paths(&config, Some(&main_dir), |repo_ref| {
            assert_eq!(repo_ref.project_path, "anthropics/skills");
            assert_eq!(repo_ref.host, default_github_host());
            Ok(repo_root.clone())
        })
        .expect("discover skills");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].install_dir_name, "writing-plans");
    }

    #[test]
    fn discover_available_returns_empty_for_empty_repos() {
        let mut config = AppConfig::default();
        config.skill_repos.clear();
        let temp = tempfile::tempdir().expect("tempdir");
        let (skills, warnings) =
            discover_available_with_warnings(&config, None, temp.path(), false);

        assert!(warnings.is_empty());
        assert!(skills.is_empty());
    }

    #[test]
    fn deduplicate_discoverable_skills_keeps_first_occurrence() {
        let skill = DiscoverableSkill {
            key: "github.com/anthropics/skills:skills/brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "Explore ideas.".to_string(),
            directory: "skills/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            repo_host: default_github_host(),
            project_path: "anthropics/skills".to_string(),
            repo_owner: "anthropics".to_string(),
            repo_name: "skills".to_string(),
            repo_branch: "main".to_string(),
            source: "github".to_string(),
            ..Default::default()
        };
        let duplicate = skill.clone();

        let deduped = deduplicate_discoverable_skills(vec![skill.clone(), duplicate]);

        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0], skill);
    }

    #[test]
    fn merge_repo_into_discover_cache_replaces_existing_repo_entries() {
        let github_skill = DiscoverableSkill {
            key: "github.com/obra/superpowers:skills/brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "Existing GitHub skill.".to_string(),
            directory: "skills/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            repo_host: default_github_host(),
            project_path: "obra/superpowers".to_string(),
            repo_owner: "obra".to_string(),
            repo_name: "superpowers".to_string(),
            repo_branch: "main".to_string(),
            source: "github".to_string(),
            ..Default::default()
        };
        let old_gitlab_skill = DiscoverableSkill {
            key: "git.example.com/group/project:skills/old-skill".to_string(),
            name: "old-skill".to_string(),
            description: "Stale GitLab cache.".to_string(),
            directory: "skills/old-skill".to_string(),
            install_dir_name: "old-skill".to_string(),
            repo_host: "git.example.com".to_string(),
            project_path: "group/project".to_string(),
            repo_owner: "group".to_string(),
            repo_name: "project".to_string(),
            repo_branch: "main".to_string(),
            source: "gitlab".to_string(),
            ..Default::default()
        };
        let gitlab_repo = SkillRepo {
            host: "git.example.com".to_string(),
            provider: "gitlab".to_string(),
            project_path: "group/project".to_string(),
            owner: "group".to_string(),
            name: "project".to_string(),
            branch: "main".to_string(),
            enabled: false,
        };

        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![github_skill.clone(), old_gitlab_skill],
            },
            ..Default::default()
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let merged = merge_repo_into_discover_cache(
            &mut config,
            &gitlab_repo,
            None,
            temp.path(),
            false,
        )
            .expect("merge cache");

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].key, github_skill.key);
        assert_eq!(config.skill_discover_cache.skills, merged);
    }

    #[test]
    fn remove_repo_from_discover_cache_does_not_download() {
        let mut config = AppConfig {
            skill_discover_cache: SkillDiscoverCache {
                fetched_at: None,
                skills: vec![
                    DiscoverableSkill {
                        key: "github.com/obra/superpowers:skills/brainstorming".to_string(),
                        name: "brainstorming".to_string(),
                        description: "Keep me.".to_string(),
                        directory: "skills/brainstorming".to_string(),
                        install_dir_name: "brainstorming".to_string(),
                        repo_host: default_github_host(),
                        project_path: "obra/superpowers".to_string(),
                        repo_owner: "obra".to_string(),
                        repo_name: "superpowers".to_string(),
                        repo_branch: "main".to_string(),
                        source: "github".to_string(),
                        ..Default::default()
                    },
                    DiscoverableSkill {
                        key: "git.example.com/group/project:skills/gitlab-skill".to_string(),
                        name: "gitlab-skill".to_string(),
                        description: "Remove me.".to_string(),
                        directory: "skills/gitlab-skill".to_string(),
                        install_dir_name: "gitlab-skill".to_string(),
                        repo_host: "git.example.com".to_string(),
                        project_path: "group/project".to_string(),
                        repo_owner: "group".to_string(),
                        repo_name: "project".to_string(),
                        repo_branch: "main".to_string(),
                        source: "gitlab".to_string(),
                        ..Default::default()
                    },
                ],
            },
            ..Default::default()
        };

        let remaining =
            remove_repo_from_discover_cache(&mut config, "git.example.com", "group/project");

        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].install_dir_name, "brainstorming");
        assert_eq!(config.skill_discover_cache.skills, remaining);
    }

    #[test]
    fn discover_available_with_warnings_skips_failed_repo() {
        let temp = tempfile::tempdir().expect("tempdir");
        let gitlab_root = temp.path().join("gitlab-repo");
        write_valid_skill(
            &gitlab_root.join("skills").join("gitlab-skill"),
            "gitlab-skill",
            "From GitLab.",
        );

        let config = AppConfig {
            skill_repos: vec![
                enabled_repo("obra", "superpowers"),
                SkillRepo {
                    host: "git.example.com".to_string(),
                    provider: "gitlab".to_string(),
                    project_path: "group/project".to_string(),
                    owner: "group".to_string(),
                    name: "project".to_string(),
                    branch: "main".to_string(),
                    enabled: true,
                },
            ],
            ..Default::default()
        };

        let (skills, warnings) = discover_available_with_warnings_from_paths(&config, None, |repo_ref| {
            if repo_ref.host == default_github_host() {
                Err(AppError::Io {
                    path: None,
                    message: "network unreachable".to_string(),
                })
            } else {
                Ok(gitlab_root.clone())
            }
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("github.com/obra/superpowers"));
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].install_dir_name, "gitlab-skill");
    }

    #[test]
    fn filter_uninstalled_keeps_hub_skill_when_only_other_source_shares_link_name() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let hub_skill = DiscoverableSkill {
            key: "oxygen-skill-hub:common/talos-lecture-json-review".to_string(),
            source: "skillhub".to_string(),
            storage_key: "hub/oxygen-skill-hub/common/talos-lecture-json-review".to_string(),
            link_name: "talos-lecture-json-review".to_string(),
            install_dir_name: "talos-lecture-json-review".to_string(),
            hub_endpoint_id: "oxygen-skill-hub".to_string(),
            hub_skill_group: "common".to_string(),
            hub_skill_id: "talos-lecture-json-review".to_string(),
            ..Default::default()
        };

        let mut records = HashMap::new();
        records.insert(
            "repo/git.xkw.cn--mp-oxygen-uc-skills/talos-lecture-json-review".to_string(),
            SkillRecord {
                link_name: "talos-lecture-json-review".to_string(),
                storage_key: "repo/git.xkw.cn--mp-oxygen-uc-skills/talos-lecture-json-review"
                    .to_string(),
                ..Default::default()
            },
        );

        let filtered = filter_uninstalled_discoverable_skills(
            vec![hub_skill],
            Some(&main_dir),
            Some(&records),
        );

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn filter_uninstalled_hides_hub_skill_when_same_endpoint_group_and_id_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let hub_skill = DiscoverableSkill {
            key: "oxygen-skill-hub:review/kqs-review".to_string(),
            source: "skillhub".to_string(),
            storage_key: "hub/oxygen-skill-hub/review/kqs-review".to_string(),
            link_name: "kqs-review".to_string(),
            install_dir_name: "kqs-review".to_string(),
            hub_endpoint_id: "oxygen-skill-hub".to_string(),
            hub_skill_group: "review".to_string(),
            hub_skill_id: "kqs-review".to_string(),
            ..Default::default()
        };

        let mut records = HashMap::new();
        records.insert(
            "hub/oxygen-skill-hub/review/kqs-review".to_string(),
            SkillRecord {
                source: "skillhub".to_string(),
                link_name: "kqs-review".to_string(),
                storage_key: "hub/oxygen-skill-hub/review/kqs-review".to_string(),
                hub_endpoint_id: "oxygen-skill-hub".to_string(),
                hub_skill_group: "review".to_string(),
                hub_skill_id: "kqs-review".to_string(),
                ..Default::default()
            },
        );

        let filtered = filter_uninstalled_discoverable_skills(
            vec![hub_skill],
            Some(&main_dir),
            Some(&records),
        );

        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_uninstalled_keeps_hub_skill_when_same_id_in_different_group() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let hub_skill = DiscoverableSkill {
            key: "oxygen-skill-hub:review/kqs-review".to_string(),
            source: "skillhub".to_string(),
            storage_key: "hub/oxygen-skill-hub/review/kqs-review".to_string(),
            link_name: "kqs-review".to_string(),
            install_dir_name: "kqs-review".to_string(),
            hub_endpoint_id: "oxygen-skill-hub".to_string(),
            hub_skill_group: "review".to_string(),
            hub_skill_id: "kqs-review".to_string(),
            ..Default::default()
        };

        let mut records = HashMap::new();
        records.insert(
            "hub/oxygen-skill-hub/common/kqs-review".to_string(),
            SkillRecord {
                source: "skillhub".to_string(),
                link_name: "kqs-review".to_string(),
                storage_key: "hub/oxygen-skill-hub/common/kqs-review".to_string(),
                hub_endpoint_id: "oxygen-skill-hub".to_string(),
                hub_skill_group: "common".to_string(),
                hub_skill_id: "kqs-review".to_string(),
                ..Default::default()
            },
        );

        let filtered = filter_uninstalled_discoverable_skills(
            vec![hub_skill],
            Some(&main_dir),
            Some(&records),
        );

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn second_discover_uses_repo_cache_without_redownload() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path();
        let repo = enabled_repo("anthropics", "skills");
        let download_count = AtomicUsize::new(0);
        const SHA: &str = "abc123deadbeef";

        let fetch_sha = |_repo_ref: &RepoRef| Ok(SHA.to_string());
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_valid_skill(
                &dest_dir.join("skills").join("writing-plans"),
                "writing-plans",
                "Create implementation plans.",
            );
            Ok(())
        };

        for _ in 0..2 {
            let repo_root = repo_cache::ensure_repo_tree_with_hooks(
                &repo,
                app_data,
                false,
                fetch_sha,
                download_repo,
            )
            .expect("ensure repo tree");
            let skills = fetch_repo_skills_from_path(&repo_root, &repo.to_repo_ref());
            assert_eq!(skills.len(), 1);
            assert_eq!(skills[0].install_dir_name, "writing-plans");
        }

        assert_eq!(download_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn force_discover_still_uses_cache_when_sha_unchanged() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().expect("tempdir");
        let app_data = temp.path();
        let repo = enabled_repo("anthropics", "skills");
        let download_count = AtomicUsize::new(0);
        const SHA: &str = "abc123deadbeef";

        let fetch_sha = |_repo_ref: &RepoRef| Ok(SHA.to_string());
        let download_repo = |dest_dir: &Path, _repo_ref: &RepoRef| {
            download_count.fetch_add(1, Ordering::SeqCst);
            write_valid_skill(
                &dest_dir.join("skills").join("writing-plans"),
                "writing-plans",
                "Create implementation plans.",
            );
            Ok(())
        };

        repo_cache::ensure_repo_tree_with_hooks(&repo, app_data, false, fetch_sha, download_repo)
            .expect("seed cache");
        repo_cache::ensure_repo_tree_with_hooks(&repo, app_data, true, fetch_sha, download_repo)
            .expect("force discover");
        assert_eq!(download_count.load(Ordering::SeqCst), 1);
    }

    fn discover_available_with_warnings_from_paths<F>(
        config: &AppConfig,
        main_dir: Option<&Path>,
        mut fetch_repo: F,
    ) -> (Vec<DiscoverableSkill>, Vec<String>)
    where
        F: FnMut(&RepoRef) -> Result<PathBuf, AppError>,
    {
        let mut skills = Vec::new();
        let mut warnings = Vec::new();

        for repo in config.skill_repos.iter().filter(|repo| repo.enabled) {
            let repo_ref = repo.to_repo_ref();
            let repo_label = repo_display_label(repo);
            match fetch_repo(&repo_ref) {
                Ok(repo_root) => {
                    skills.extend(fetch_repo_skills_from_path(&repo_root, &repo_ref));
                }
                Err(err) => {
                    warnings.push(format!("跳过来源 {}：{}", repo_label, err.to_dto().message));
                }
            }
        }

        let filtered = skills
            .into_iter()
            .filter(|skill| !is_skill_installed(skill, main_dir))
            .collect();

        (deduplicate_discoverable_skills(filtered), warnings)
    }

    fn discover_available_from_paths<F>(
        config: &AppConfig,
        main_dir: Option<&Path>,
        mut fetch_repo: F,
    ) -> Result<Vec<DiscoverableSkill>, AppError>
    where
        F: FnMut(&RepoRef) -> Result<PathBuf, AppError>,
    {
        let mut skills = Vec::new();

        for repo in config.skill_repos.iter().filter(|repo| repo.enabled) {
            let repo_ref = repo.to_repo_ref();
            let repo_root = fetch_repo(&repo_ref)?;
            skills.extend(fetch_repo_skills_from_path(&repo_root, &repo_ref));
        }

        let filtered = skills
            .into_iter()
            .filter(|skill| !is_skill_installed(skill, main_dir))
            .collect();

        Ok(deduplicate_discoverable_skills(filtered))
    }
}
