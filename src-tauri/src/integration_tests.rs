#[cfg(test)]
mod tests {
    use crate::fs_adapter;
    use crate::link_installer::{install_skill, uninstall_skill};
    use crate::models::AppError;
    use crate::skill_library::list_skills;
    use crate::skill_remover::delete_main_skill;
    use crate::test_support::fixtures::{
        build_config, create_invalid_skill, create_target_dir, create_valid_skill,
    };
    use std::fs;

    #[test]
    fn full_end_to_end_flow() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");

        // 1. Create a valid skill
        let skill = create_valid_skill(&main_dir, "brainstorming");

        // 2. Create two target dirs
        let target_a_dir = create_target_dir(temp.path(), "a");
        let target_b_dir = create_target_dir(temp.path(), "b");

        // 3. Build config
        let mut config = build_config(
            Some(&main_dir),
            &[
                ("target-a".to_string(), target_a_dir.clone()),
                ("target-b".to_string(), target_b_dir.clone()),
            ],
        );

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        assert_eq!(skills.len(), 1);
        assert!(skills[0].valid);

        // 4. Install skill into both targets
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install a");
        install_skill(&mut config, "target-b", "brainstorming", &skills).expect("install b");

        let link_a = target_a_dir.join("brainstorming");
        let link_b = target_b_dir.join("brainstorming");

        // Assert both target links exist and config has two installation records
        assert!(fs_adapter::path_exists(&link_a));
        assert!(fs_adapter::path_exists(&link_b));
        assert_eq!(config.installations.len(), 2);

        // 5. Modify source file and confirm target links see the change (read through link)
        fs::write(skill.path.join("SKILL.md"), "---\nname: brainstorming\ndescription: Updated.\n---\n")
            .expect("update source");
        assert_eq!(
            fs::read_to_string(link_a.join("SKILL.md")).unwrap(),
            "---\nname: brainstorming\ndescription: Updated.\n---\n"
        );
        assert_eq!(
            fs::read_to_string(link_b.join("SKILL.md")).unwrap(),
            "---\nname: brainstorming\ndescription: Updated.\n---\n"
        );

        // 6. Uninstall from target A; assert target A link is gone, source and target B remain
        uninstall_skill(&mut config, "target-a", "brainstorming").expect("uninstall a");
        assert!(!fs_adapter::path_exists(&link_a));
        assert!(fs_adapter::path_exists(&link_b));
        assert!(skill.path.is_dir());
        assert!(skill.path.join("SKILL.md").is_file());
        assert_eq!(config.installations.len(), 1);

        // 7. Create a real directory at target A's skill path; assert reinstall is blocked as conflict
        fs::create_dir_all(&link_a).expect("create real dir at link_a");
        fs::write(link_a.join("existing.txt"), "existing").expect("write file");
        let error = install_skill(&mut config, "target-a", "brainstorming", &skills)
            .expect_err("reinstall should conflict");
        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(link_a.join("existing.txt").is_file());

        // 8. Delete the main skill; assert both recorded links are cleaned and source is gone
        let result = delete_main_skill(&mut config, "brainstorming", true).expect("delete main skill");
        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert_eq!(result.removed_link_count, 1); // only target-b link was recorded
        assert!(!skill.path.exists());
        assert!(!fs_adapter::path_exists(&link_b));
        assert!(config.installations.is_empty());
    }

    #[test]
    fn config_default_load_save_malformed_behavior() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.json");
        let store = crate::config_store::ConfigStore::new(config_path.clone());

        // Missing config returns default
        let config = store.load().expect("load default");
        assert_eq!(config.version, 1);
        assert!(config.settings.main_skills_dir.is_none());

        // Save and load round-trips
        let mut to_save = config.clone();
        to_save.settings.main_skills_dir = Some(temp.path().join("main"));
        store.save(&to_save).expect("save");
        let loaded = store.load().expect("load saved");
        assert_eq!(loaded.settings.main_skills_dir, to_save.settings.main_skills_dir);

        // Malformed config returns error without overwriting
        fs::write(&config_path, "{not json").expect("write malformed");
        let error = store.load().expect_err("malformed should fail");
        assert!(matches!(error, AppError::ConfigParse { .. }));
        assert_eq!(fs::read_to_string(&config_path).unwrap(), "{not json");
    }

    #[test]
    fn skill_validation_cases() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");

        // Valid skill
        let _valid = create_valid_skill(&main_dir, "valid-skill");

        // Missing file
        let _missing_file = create_invalid_skill(&main_dir, "missing-file");

        // Missing name
        let missing_name_dir = main_dir.join("missing-name");
        fs::create_dir_all(&missing_name_dir).expect("create dir");
        fs::write(
            missing_name_dir.join("SKILL.md"),
            "---\ndescription: Has a description.\n---\n",
        )
        .expect("write skill md");

        // Missing description
        let missing_desc_dir = main_dir.join("missing-desc");
        fs::create_dir_all(&missing_desc_dir).expect("create dir");
        fs::write(
            missing_desc_dir.join("SKILL.md"),
            "---\nname: Missing Description\n---\n",
        )
        .expect("write skill md");

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        assert_eq!(skills.len(), 4);

        let valid = skills.iter().find(|s| s.dir_name == "valid-skill").unwrap();
        assert!(valid.valid);

        let missing_file = skills.iter().find(|s| s.dir_name == "missing-file").unwrap();
        assert!(!missing_file.valid);
        assert!(missing_file.validation_errors.contains(&"Missing SKILL.md".to_string()));

        let missing_name = skills.iter().find(|s| s.dir_name == "missing-name").unwrap();
        assert!(!missing_name.valid);
        assert!(missing_name
            .validation_errors
            .contains(&"Missing frontmatter.name".to_string()));

        let missing_desc = skills.iter().find(|s| s.dir_name == "missing-desc").unwrap();
        assert!(!missing_desc.valid);
        assert!(missing_desc
            .validation_errors
            .contains(&"Missing frontmatter.description".to_string()));
    }
}
