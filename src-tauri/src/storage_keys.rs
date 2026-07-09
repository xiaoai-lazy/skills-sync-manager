use crate::models::{AppConfig, Installation};
use std::path::{Component, Path};

pub fn reconcile_storage_keys(config: &mut AppConfig) -> bool {
    let Some(main_dir) = config.settings.main_skills_dir.as_deref() else {
        return false;
    };

    let skill_records = config.skill_records.clone();
    let mut changed = false;
    for installation in &mut config.installations {
        if !installation.skill_storage_key.is_empty() {
            continue;
        }
        if let Some(key) =
            infer_storage_key_for_installation(&skill_records, installation, main_dir)
        {
            installation.skill_storage_key = key;
            changed = true;
        }
    }
    changed
}

fn infer_storage_key_for_installation(
    skill_records: &std::collections::HashMap<String, crate::models::SkillRecord>,
    installation: &Installation,
    main_dir: &Path,
) -> Option<String> {
    for record in skill_records.values() {
        if record.link_name == installation.skill_dir_name
            || record.storage_key == installation.skill_dir_name
        {
            if !record.storage_key.is_empty() {
                return Some(record.storage_key.clone());
            }
        }
    }

    storage_key_from_source_path(main_dir, &installation.source_path)
}

fn storage_key_from_source_path(main_dir: &Path, source_path: &Path) -> Option<String> {
    let relative = source_path.strip_prefix(main_dir).ok()?;
    let key = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AppConfig, Installation, LinkType, SkillRecord, Target,
    };

    #[test]
    fn reconcile_fills_storage_key_from_skill_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        std::fs::create_dir_all(&main_dir).expect("create main");

        let storage_key = "repo/github.com--obra-superpowers/brainstorming";
        let source_path = main_dir.join("repo").join("github.com--obra-superpowers").join("brainstorming");

        let mut config = AppConfig {
            settings: crate::models::Settings {
                main_skills_dir: Some(main_dir.clone()),
                link_strategy: crate::models::LinkStrategy::Auto,
            },
            installations: vec![Installation {
                id: "install-1".into(),
                skill_dir_name: "brainstorming".into(),
                skill_name: "brainstorming".into(),
                source_path: source_path.clone(),
                target_id: "target-1".into(),
                link_path: temp.path().join("target").join("brainstorming"),
                link_type: LinkType::Junction,
                created_at: "1".into(),
                skill_storage_key: String::new(),
            }],
            skill_records: [(
                storage_key.to_string(),
                SkillRecord {
                    storage_key: storage_key.to_string(),
                    link_name: "brainstorming".to_string(),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            targets: vec![Target::global_custom(
                "target-1",
                "Target",
                temp.path().join("target"),
                "1",
                "1",
            )],
            ..Default::default()
        };

        assert!(reconcile_storage_keys(&mut config));
        assert_eq!(
            config.installations[0].skill_storage_key,
            storage_key
        );
    }

    #[test]
    fn reconcile_infers_storage_key_from_source_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main");
        let source_path = main_dir.join("repo").join("github.com--foo-bar").join("tdd");
        std::fs::create_dir_all(&source_path).expect("create source");

        let mut config = AppConfig {
            settings: crate::models::Settings {
                main_skills_dir: Some(main_dir.clone()),
                link_strategy: crate::models::LinkStrategy::Auto,
            },
            installations: vec![Installation {
                id: "install-1".into(),
                skill_dir_name: "tdd".into(),
                skill_name: "tdd".into(),
                source_path,
                target_id: "target-1".into(),
                link_path: temp.path().join("target").join("tdd"),
                link_type: LinkType::Junction,
                created_at: "1".into(),
                skill_storage_key: String::new(),
            }],
            ..Default::default()
        };

        assert!(reconcile_storage_keys(&mut config));
        assert_eq!(
            config.installations[0].skill_storage_key,
            "repo/github.com--foo-bar/tdd"
        );
    }
}
