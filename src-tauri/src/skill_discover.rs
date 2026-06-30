use crate::models::{AppConfig, AppError, DiscoverableSkill};
use crate::skill_downloader;
use crate::skill_library;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn discover_available(
    config: &AppConfig,
    main_dir: Option<&Path>,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    let installed = existing_install_dir_names(main_dir);
    let mut skills = Vec::new();

    for repo in config.skill_repos.iter().filter(|repo| repo.enabled) {
        let repo_root = skill_downloader::download_repo(&repo.owner, &repo.name, &repo.branch)?;
        skills.extend(fetch_repo_skills_from_path(
            &repo_root,
            &repo.owner,
            &repo.name,
            &repo.branch,
        ));
    }

    let filtered = skills
        .into_iter()
        .filter(|skill| !installed.contains(&skill.install_dir_name.to_lowercase()))
        .collect();

    Ok(deduplicate_discoverable_skills(filtered))
}

pub fn fetch_repo_skills_from_path(
    repo_root: &Path,
    owner: &str,
    name: &str,
    branch: &str,
) -> Vec<DiscoverableSkill> {
    let mut skills = Vec::new();
    scan_dir_recursive(repo_root, repo_root, owner, name, branch, &mut skills);
    skills
}

fn scan_dir_recursive(
    repo_root: &Path,
    current: &Path,
    owner: &str,
    name: &str,
    branch: &str,
    skills: &mut Vec<DiscoverableSkill>,
) {
    let skill_md = current.join("SKILL.md");
    if skill_md.is_file() {
        if let Ok(raw) = fs::read_to_string(&skill_md) {
            if let Some(metadata) = skill_library::parse_valid_skill_metadata(&raw) {
                let directory = relative_directory(repo_root, current);
                let install_dir_name = install_dir_name_from_directory(&directory);
                let key = format!("{}/{}:{}", owner, name, directory);

                skills.push(DiscoverableSkill {
                    key,
                    name: metadata.name,
                    description: metadata.description,
                    directory,
                    install_dir_name,
                    repo_owner: owner.to_string(),
                    repo_name: name.to_string(),
                    repo_branch: branch.to_string(),
                    source: "github".to_string(),
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
            scan_dir_recursive(repo_root, &path, owner, name, branch, skills);
        }
    }
}

pub fn deduplicate_discoverable_skills(skills: Vec<DiscoverableSkill>) -> Vec<DiscoverableSkill> {
    let mut seen = HashSet::new();
    skills
        .into_iter()
        .filter(|skill| seen.insert(skill.key.clone()))
        .collect()
}

fn existing_install_dir_names(main_dir: Option<&Path>) -> HashSet<String> {
    let Some(main_dir) = main_dir else {
        return HashSet::new();
    };

    if !main_dir.is_dir() {
        return HashSet::new();
    }

    let mut names = HashSet::new();
    let entries = match fs::read_dir(main_dir) {
        Ok(entries) => entries,
        Err(_) => return names,
    };

    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.insert(name.to_lowercase());
            }
        }
    }

    names
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
    use crate::models::SkillRepo;
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
            owner: owner.to_string(),
            name: name.to_string(),
            branch: "main".to_string(),
            enabled: true,
        }
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

        let skills = fetch_repo_skills_from_path(temp.path(), "anthropics", "skills", "main");

        assert_eq!(skills.len(), 2);
        assert!(skills.iter().any(|skill| skill.install_dir_name == "brainstorming"));
        assert!(skills.iter().any(|skill| skill.install_dir_name == "writing-plans"));
        assert!(skills
            .iter()
            .any(|skill| skill.key == "anthropics/skills:skills/brainstorming"));
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

        let skills = discover_available_from_paths(&config, Some(&main_dir), |repo| {
            assert_eq!(repo.owner, "anthropics");
            assert_eq!(repo.name, "skills");
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
        let skills =
            discover_available(&config, None).expect("empty repos should succeed without error");

        assert!(skills.is_empty());
    }

    #[test]
    fn deduplicate_discoverable_skills_keeps_first_occurrence() {
        let skill = DiscoverableSkill {
            key: "anthropics/skills:skills/brainstorming".to_string(),
            name: "brainstorming".to_string(),
            description: "Explore ideas.".to_string(),
            directory: "skills/brainstorming".to_string(),
            install_dir_name: "brainstorming".to_string(),
            repo_owner: "anthropics".to_string(),
            repo_name: "skills".to_string(),
            repo_branch: "main".to_string(),
            source: "github".to_string(),
        };
        let duplicate = skill.clone();

        let deduped = deduplicate_discoverable_skills(vec![skill.clone(), duplicate]);

        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0], skill);
    }

    fn discover_available_from_paths<F>(
        config: &AppConfig,
        main_dir: Option<&Path>,
        mut fetch_repo: F,
    ) -> Result<Vec<DiscoverableSkill>, AppError>
    where
        F: FnMut(&SkillRepo) -> Result<PathBuf, AppError>,
    {
        let installed = existing_install_dir_names(main_dir);
        let mut skills = Vec::new();

        for repo in config.skill_repos.iter().filter(|repo| repo.enabled) {
            let repo_root = fetch_repo(repo)?;
            skills.extend(fetch_repo_skills_from_path(
                &repo_root,
                &repo.owner,
                &repo.name,
                &repo.branch,
            ));
        }

        let filtered = skills
            .into_iter()
            .filter(|skill| !installed.contains(&skill.install_dir_name.to_lowercase()))
            .collect();

        Ok(deduplicate_discoverable_skills(filtered))
    }
}
