#[cfg(test)]
pub mod fixtures {
    use crate::models::{AppConfig, Installation, Settings, SkillView, Target};
    use std::fs;
    use std::path::{Path, PathBuf};

    pub fn create_valid_skill(main_dir: &Path, dir_name: &str) -> SkillView {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
                dir_name
            ),
        )
        .expect("write skill md");
        SkillView {
            dir_name: dir_name.to_string(),
            name: Some(dir_name.to_string()),
            description: Some("Test skill.".to_string()),
            path: skill_dir,
            valid: true,
            validation_errors: Vec::new(),
        }
    }

    pub fn create_invalid_skill(main_dir: &Path, dir_name: &str, _reason: &str) -> SkillView {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        SkillView {
            dir_name: dir_name.to_string(),
            name: None,
            description: None,
            path: skill_dir,
            valid: false,
            validation_errors: vec!["Missing SKILL.md".to_string()],
        }
    }

    pub fn create_target_dir(temp: &Path, target_id: &str) -> PathBuf {
        let target_dir = temp.join(format!("target-{}", target_id));
        fs::create_dir_all(&target_dir).expect("create target dir");
        target_dir
    }

    pub fn build_config(
        main_dir: Option<&Path>,
        targets: &[(String, PathBuf)],
    ) -> AppConfig {
        AppConfig {
            version: 1,
            settings: Settings {
                main_skills_dir: main_dir.map(Path::to_path_buf),
                link_strategy: crate::models::LinkStrategy::Auto,
            },
            targets: targets
                .iter()
                .enumerate()
                .map(|(index, (id, skills_dir))| Target {
                    id: id.clone(),
                    name: format!("Target {}", index + 1),
                    skills_dir: skills_dir.clone(),
                    created_at: "1".to_string(),
                    updated_at: "1".to_string(),
                })
                .collect(),
            installations: Vec::new(),
        }
    }

    pub fn add_installation_record(
        config: &mut AppConfig,
        target_id: &str,
        skill: &SkillView,
        link_path: &Path,
    ) {
        let installation = Installation {
            id: format!("install-{}", config.installations.len()),
            skill_dir_name: skill.dir_name.clone(),
            skill_name: skill.name.clone().unwrap_or_else(|| skill.dir_name.clone()),
            source_path: skill.path.clone(),
            target_id: target_id.to_string(),
            link_path: link_path.to_path_buf(),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "1".to_string(),
        };
        config.installations.push(installation);
    }
}
