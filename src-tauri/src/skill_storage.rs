use std::path::{Path, PathBuf};

pub fn skill_id_from_directory(directory: &str) -> String {
    directory
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(directory)
        .to_string()
}

pub fn sanitize_slug_part(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in input.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized == '/'
            || normalized == ' '
            || normalized == '\\'
            || normalized == ':'
            || normalized == '*'
            || normalized == '?'
            || normalized == '"'
            || normalized == '<'
            || normalized == '>'
            || normalized == '|'
            || normalized.is_control()
        {
            if !last_was_dash && !slug.is_empty() {
                slug.push('-');
                last_was_dash = true;
            }
        } else if normalized == '-' {
            if !last_was_dash && !slug.is_empty() {
                slug.push('-');
                last_was_dash = true;
            }
        } else {
            slug.push(normalized);
            last_was_dash = false;
        }
    }

    if slug.is_empty() {
        "unknown".to_string()
    } else {
        slug
    }
}

pub fn compute_repo_slug(host: &str, project_path: &str) -> String {
    format!(
        "{}--{}",
        sanitize_slug_part(host),
        sanitize_slug_part(project_path)
    )
}

pub fn storage_key_for_repo(repo_slug: &str, skill_id: &str) -> String {
    format!("repo/{repo_slug}/{skill_id}")
}

pub fn storage_key_for_hub(hub_endpoint_id: &str, group: &str, skill_id: &str) -> String {
    format!("hub/{hub_endpoint_id}/{group}/{skill_id}")
}

pub fn storage_key_for_local(skill_id: &str) -> String {
    format!("local/{skill_id}")
}

pub fn main_library_path(main_dir: &Path, storage_key: &str) -> PathBuf {
    storage_key
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(main_dir.to_path_buf(), |path, part| path.join(part))
}

pub fn storage_key_from_record_source(
    source: &str,
    repo_slug: Option<&str>,
    hub_endpoint_id: Option<&str>,
    hub_group: Option<&str>,
    skill_id: &str,
) -> String {
    match source {
        "github" | "gitlab" | "skillssh" => storage_key_for_repo(
            repo_slug.unwrap_or("unknown"),
            skill_id,
        ),
        "skillhub" => storage_key_for_hub(
            hub_endpoint_id.unwrap_or("unknown"),
            hub_group.unwrap_or("common"),
            skill_id,
        ),
        _ => storage_key_for_local(skill_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_repo_slug_github() {
        assert_eq!(
            compute_repo_slug("github.com", "anthropics/skills"),
            "github.com--anthropics-skills"
        );
    }

    #[test]
    fn compute_repo_slug_gitlab_host() {
        assert_eq!(
            compute_repo_slug("gitlab.example.com", "team/tools"),
            "gitlab.example.com--team-tools"
        );
    }

    #[test]
    fn storage_key_for_repo_uses_skill_id() {
        let slug = compute_repo_slug("github.com", "anthropics/skills");
        assert_eq!(
            storage_key_for_repo(&slug, "tdd"),
            "repo/github.com--anthropics-skills/tdd"
        );
    }

    #[test]
    fn storage_key_for_hub() {
        assert_eq!(
            super::storage_key_for_hub("company-hub", "common", "brainstorming"),
            "hub/company-hub/common/brainstorming"
        );
    }

    #[test]
    fn storage_key_for_local() {
        assert_eq!(
            super::storage_key_for_local("my-skill"),
            "local/my-skill"
        );
    }

    #[test]
    fn sanitize_empty_becomes_unknown() {
        assert_eq!(sanitize_slug_part(""), "unknown");
    }

    #[test]
    fn skill_id_from_directory() {
        assert_eq!(super::skill_id_from_directory("skills/tdd"), "tdd");
        assert_eq!(super::skill_id_from_directory("tdd"), "tdd");
    }
}
