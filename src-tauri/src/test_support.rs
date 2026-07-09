#[cfg(test)]
pub mod fixtures {
    use crate::models::{AppConfig, Settings, SkillView, Target};
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
            link_name: dir_name.to_string(),
            ..Default::default()
        }
    }

    pub fn create_invalid_skill(main_dir: &Path, dir_name: &str) -> SkillView {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        SkillView {
            dir_name: dir_name.to_string(),
            name: None,
            description: None,
            path: skill_dir,
            valid: false,
            validation_errors: vec!["Missing SKILL.md".to_string()],
            link_name: dir_name.to_string(),
            ..Default::default()
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
                .map(|(index, (id, skills_dir))| {
                    Target::global_custom(
                        id.clone(),
                        format!("Target {}", index + 1),
                        skills_dir.clone(),
                        "1",
                        "1",
                    )
                })
                .collect(),
            installations: Vec::new(),
            ..Default::default()
        }
    }
}
