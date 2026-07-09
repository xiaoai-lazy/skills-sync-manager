use crate::models::{AppError, SkillView};
use crate::skill_storage;
use serde::Deserialize;
use std::fs;
use std::path::{Component, Path};

const MISSING_SKILL_MD: &str = "Missing SKILL.md";
const MISSING_FRONTMATTER: &str = "Missing frontmatter";
const MISSING_FRONTMATTER_NAME: &str = "Missing frontmatter.name";
const MISSING_FRONTMATTER_DESCRIPTION: &str = "Missing frontmatter.description";

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SkillMetadata {
    name: Option<String>,
    description: Option<String>,
}

pub fn list_skills(main_dir: Option<&Path>) -> Result<Vec<SkillView>, AppError> {
    let Some(main_dir) = main_dir else {
        return Ok(Vec::new());
    };

    if !main_dir.exists() {
        return Err(AppError::InvalidMainSkillsDir {
            path: main_dir.to_path_buf(),
            message: "Main skills directory does not exist".to_string(),
        });
    }

    if !main_dir.is_dir() {
        return Err(AppError::InvalidMainSkillsDir {
            path: main_dir.to_path_buf(),
            message: "Main skills directory is not a directory".to_string(),
        });
    }

    let mut skills = Vec::new();
    collect_skills(main_dir, main_dir, &mut skills)?;
    skills.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    Ok(skills)
}

fn collect_skills(
    main_dir: &Path,
    current_dir: &Path,
    skills: &mut Vec<SkillView>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(current_dir).map_err(|err| AppError::Io {
        path: Some(current_dir.to_path_buf()),
        message: err.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| AppError::Io {
            path: Some(current_dir.to_path_buf()),
            message: err.to_string(),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| AppError::Io {
            path: Some(path.clone()),
            message: err.to_string(),
        })?;

        if !file_type.is_dir() {
            continue;
        }

        if is_hidden_dir(&path) {
            continue;
        }

        let is_top_level = current_dir == main_dir;
        let has_skill_md = path.join("SKILL.md").exists();

        if has_skill_md {
            skills.push(validate_skill_dir(main_dir, &path)?);
        } else if is_top_level && !has_descendant_skill_leaf(&path)? {
            skills.push(validate_skill_dir(main_dir, &path)?);
        }

        collect_skills(main_dir, &path, skills)?;
    }

    Ok(())
}

fn has_descendant_skill_leaf(dir: &Path) -> Result<bool, AppError> {
    let entries = fs::read_dir(dir).map_err(|err| AppError::Io {
        path: Some(dir.to_path_buf()),
        message: err.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| AppError::Io {
            path: Some(dir.to_path_buf()),
            message: err.to_string(),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| AppError::Io {
            path: Some(path.clone()),
            message: err.to_string(),
        })?;

        if !file_type.is_dir() || is_hidden_dir(&path) {
            continue;
        }

        if path.join("SKILL.md").exists() || has_descendant_skill_leaf(&path)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_hidden_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn storage_key_for_skill_dir(main_dir: &Path, skill_dir: &Path) -> String {
    let relative = skill_dir
        .strip_prefix(main_dir)
        .unwrap_or(skill_dir)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    relative
}

fn validate_skill_dir(main_dir: &Path, skill_dir: &Path) -> Result<SkillView, AppError> {
    let storage_key = storage_key_for_skill_dir(main_dir, skill_dir);
    let link_name = skill_storage::skill_id_from_directory(&storage_key);
    let dir_name = link_name.clone();
    let skill_md_path = skill_dir.join("SKILL.md");

    if !skill_md_path.exists() {
        return Ok(SkillView {
            dir_name,
            name: None,
            description: None,
            path: skill_dir.to_path_buf(),
            valid: false,
            validation_errors: vec![MISSING_SKILL_MD.to_string()],
            storage_key,
            link_name,
        });
    }

    let raw = fs::read_to_string(&skill_md_path).map_err(|err| AppError::Io {
        path: Some(skill_md_path),
        message: err.to_string(),
    })?;

    match parse_skill_frontmatter(&raw) {
        Ok(metadata) => Ok(SkillView {
            dir_name,
            name: trim_non_empty(metadata.name),
            description: trim_non_empty(metadata.description),
            path: skill_dir.to_path_buf(),
            valid: true,
            validation_errors: Vec::new(),
            storage_key,
            link_name,
        }),
        Err(validation_errors) => Ok(SkillView {
            dir_name,
            name: None,
            description: None,
            path: skill_dir.to_path_buf(),
            valid: false,
            validation_errors,
            storage_key,
            link_name,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSkillMetadata {
    pub name: String,
    pub description: String,
}

pub fn parse_valid_skill_metadata(raw: &str) -> Option<ParsedSkillMetadata> {
    match parse_skill_frontmatter(raw) {
        Ok(metadata) => {
            let name = trim_non_empty(metadata.name)?;
            let description = trim_non_empty(metadata.description)?;
            Some(ParsedSkillMetadata { name, description })
        }
        Err(_) => None,
    }
}

fn parse_skill_frontmatter(raw: &str) -> Result<SkillMetadata, Vec<String>> {
    let Some(after_opening_delimiter) = raw.strip_prefix("---") else {
        return Err(vec![MISSING_FRONTMATTER.to_string()]);
    };

    let after_opening_delimiter = after_opening_delimiter
        .strip_prefix("\r\n")
        .or_else(|| after_opening_delimiter.strip_prefix("\n"))
        .ok_or_else(|| vec![MISSING_FRONTMATTER.to_string()])?;

    let Some((frontmatter, _body)) = split_frontmatter(after_opening_delimiter) else {
        return Err(vec![MISSING_FRONTMATTER.to_string()]);
    };

    let metadata: SkillMetadata = serde_yaml::from_str(frontmatter).unwrap_or(SkillMetadata {
        name: None,
        description: None,
    });

    let mut validation_errors = Vec::new();
    if is_blank(metadata.name.as_deref()) {
        validation_errors.push(MISSING_FRONTMATTER_NAME.to_string());
    }
    if is_blank(metadata.description.as_deref()) {
        validation_errors.push(MISSING_FRONTMATTER_DESCRIPTION.to_string());
    }

    if validation_errors.is_empty() {
        Ok(metadata)
    } else {
        Err(validation_errors)
    }
}

fn split_frontmatter(raw_after_opening_delimiter: &str) -> Option<(&str, &str)> {
    let mut body_start = 0;

    for segment in raw_after_opening_delimiter.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let line = line.strip_suffix('\r').unwrap_or(line);

        if line == "---" {
            return Some((
                &raw_after_opening_delimiter[..body_start],
                &raw_after_opening_delimiter[body_start + segment.len()..],
            ));
        }

        body_start += segment.len();
    }

    None
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

fn is_blank(value: Option<&str>) -> bool {
    value.map(str::trim).unwrap_or_default().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_skill(main_dir: &Path, dir_name: &str, skill_md: Option<&str>) {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        if let Some(skill_md) = skill_md {
            fs::write(skill_dir.join("SKILL.md"), skill_md).expect("write skill md");
        }
    }

    fn create_nested_skill(main_dir: &Path, relative_path: &str, skill_md: &str) {
        let skill_dir = main_dir.join(relative_path);
        fs::create_dir_all(&skill_dir).expect("create nested skill dir");
        fs::write(skill_dir.join("SKILL.md"), skill_md).expect("write skill md");
    }

    #[test]
    fn valid_skill_is_returned_as_installable() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(
            temp.path(),
            "brainstorming",
            Some("---\nname: brainstorming\ndescription: Explore ideas.\n---\n\n# Skill\n"),
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(skills[0].valid);
        assert_eq!(skills[0].dir_name, "brainstorming");
        assert_eq!(skills[0].storage_key, "brainstorming");
        assert_eq!(skills[0].link_name, "brainstorming");
        assert_eq!(skills[0].name.as_deref(), Some("brainstorming"));
        assert_eq!(skills[0].description.as_deref(), Some("Explore ideas."));
    }

    #[test]
    fn list_skills_finds_nested_storage_key_leaf() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_nested_skill(
            temp.path(),
            "repo/github.com--anthropics-skills/tdd",
            "---\nname: tdd\ndescription: Test-driven development.\n---\n\n# Skill\n",
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(skills[0].valid);
        assert_eq!(
            skills[0].storage_key,
            "repo/github.com--anthropics-skills/tdd"
        );
        assert_eq!(skills[0].link_name, "tdd");
        assert_eq!(skills[0].dir_name, "tdd");
        assert_eq!(skills[0].name.as_deref(), Some("tdd"));
    }

    #[test]
    fn missing_skill_md_is_invalid_with_reason() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(temp.path(), "broken-skill", None);

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(!skills[0].valid);
        assert_eq!(skills[0].validation_errors, vec![MISSING_SKILL_MD]);
    }

    #[test]
    fn missing_name_is_invalid_with_reason() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(
            temp.path(),
            "missing-name",
            Some("---\ndescription: Has a description.\n---\n\n# Skill\n"),
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(!skills[0].valid);
        assert_eq!(skills[0].validation_errors, vec![MISSING_FRONTMATTER_NAME]);
    }

    #[test]
    fn missing_description_is_invalid_with_reason() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(
            temp.path(),
            "missing-description",
            Some("---\nname: Missing Description\n---\n\n# Skill\n"),
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(!skills[0].valid);
        assert_eq!(
            skills[0].validation_errors,
            vec![MISSING_FRONTMATTER_DESCRIPTION]
        );
    }

    #[test]
    fn missing_frontmatter_is_invalid_with_reason() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(temp.path(), "missing-frontmatter", Some("# Skill\n"));

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(!skills[0].valid);
        assert_eq!(skills[0].validation_errors, vec![MISSING_FRONTMATTER]);
    }

    #[test]
    fn closing_frontmatter_delimiter_at_eof_is_accepted() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(
            temp.path(),
            "eof-frontmatter",
            Some("---\nname: Test\ndescription: Test skill.\n---"),
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert!(skills[0].valid);
        assert_eq!(skills[0].name.as_deref(), Some("Test"));
        assert_eq!(skills[0].description.as_deref(), Some("Test skill."));
    }

    #[test]
    fn regular_files_in_main_directory_are_ignored() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("SKILL.md"), "not a skill directory").expect("write file");
        create_skill(
            temp.path(),
            "valid-skill",
            Some("---\nname: Valid\ndescription: Valid skill.\n---\n"),
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].dir_name, "valid-skill");
    }

    #[test]
    fn unconfigured_main_directory_returns_empty_list() {
        let skills = list_skills(None).expect("list skills without main dir");

        assert!(skills.is_empty());
    }

    #[test]
    fn missing_configured_main_directory_returns_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing_main_dir = temp.path().join("missing-main-skills");

        let error = list_skills(Some(&missing_main_dir)).expect_err("missing main dir should fail");

        assert!(matches!(error, AppError::InvalidMainSkillsDir { .. }));
        assert_eq!(
            error.to_string(),
            format!(
                "invalid main skills directory at {}: Main skills directory does not exist",
                missing_main_dir.display()
            )
        );
    }

    #[test]
    fn nested_directories_inside_a_skill_are_not_treated_as_separate_skills() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(
            temp.path(),
            "parent-skill",
            Some("---\nname: Parent\ndescription: Parent skill.\n---\n"),
        );
        fs::create_dir_all(temp.path().join("parent-skill").join("nested-assets"))
            .expect("create nested assets dir");
        fs::write(
            temp.path()
                .join("parent-skill")
                .join("nested-assets")
                .join("notes.md"),
            "# Notes\n",
        )
        .expect("write nested asset");

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].dir_name, "parent-skill");
        assert_eq!(skills[0].storage_key, "parent-skill");
    }

    #[test]
    fn nested_skill_leaves_with_skill_md_are_listed_separately() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_skill(
            temp.path(),
            "parent-skill",
            Some("---\nname: Parent\ndescription: Parent skill.\n---\n"),
        );
        create_nested_skill(
            temp.path(),
            "parent-skill/nested-skill",
            "---\nname: Nested\ndescription: Nested skill.\n---\n",
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].dir_name, "nested-skill");
        assert_eq!(skills[0].storage_key, "parent-skill/nested-skill");
        assert_eq!(skills[1].dir_name, "parent-skill");
        assert_eq!(skills[1].storage_key, "parent-skill");
    }

    #[test]
    fn intermediate_container_directories_without_skill_md_are_skipped() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_nested_skill(
            temp.path(),
            "repo/github.com--anthropics-skills/tdd",
            "---\nname: tdd\ndescription: Test-driven development.\n---\n",
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].link_name, "tdd");
    }

    #[test]
    fn hidden_directories_are_skipped() {
        let temp = tempfile::tempdir().expect("tempdir");
        create_nested_skill(
            temp.path(),
            ".git/hooks/pre-commit-skill",
            "---\nname: Hidden\ndescription: Should not appear.\n---\n",
        );
        create_skill(
            temp.path(),
            "visible-skill",
            Some("---\nname: Visible\ndescription: Visible skill.\n---\n"),
        );

        let skills = list_skills(Some(temp.path())).expect("list skills");

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].dir_name, "visible-skill");
    }
}
