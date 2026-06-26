#[cfg(test)]
mod tests {
    use crate::fs_adapter;
    use crate::link_installer::{install_skill, uninstall_skill};
    use crate::models::{AppError, Installation};
    use crate::skill_library::list_skills;
    use crate::skill_remover::delete_main_skill;
    use crate::test_support::fixtures::{
        build_config, create_invalid_skill, create_target_dir,
        create_valid_skill,
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
    fn delete_main_skill_aborts_when_recorded_link_cleanup_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");

        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_a_dir = create_target_dir(temp.path(), "a");
        let target_b_dir = create_target_dir(temp.path(), "b");

        let mut config = build_config(
            Some(&main_dir),
            &[
                ("target-a".to_string(), target_a_dir.clone()),
                ("target-b".to_string(), target_b_dir.clone()),
            ],
        );

        let skills = list_skills(Some(&main_dir)).expect("list skills");

        // Install skill into both targets
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install a");
        install_skill(&mut config, "target-b", "brainstorming", &skills).expect("install b");

        let link_a = target_a_dir.join("brainstorming");
        let link_b = target_b_dir.join("brainstorming");

        // Corrupt one target link (replace with a real directory)
        fs_adapter::remove_recorded_link(&link_b, &skill.path).expect("remove link_b");
        fs::create_dir_all(&link_b).expect("create real dir at link_b");
        fs::write(link_b.join("keep.txt"), "keep").expect("write file");

        // Assert delete_main_skill aborts
        let error = delete_main_skill(&mut config, "brainstorming", true)
            .expect_err("should abort when link cleanup fails");
        assert!(matches!(error, AppError::Io { .. }));

        // Source remains, both installation records remain
        assert!(skill.path.exists());
        assert!(skill.path.join("SKILL.md").is_file());
        assert_eq!(config.installations.len(), 2);

        // link_a was already cleaned up before the failure at link_b
        assert!(!fs_adapter::path_exists(&link_a));
        // link_b is the corrupted real directory that caused the failure
        assert!(link_b.is_dir());
        assert!(link_b.join("keep.txt").is_file());
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
        let _missing_file = create_invalid_skill(&main_dir, "missing-file", "missing skill.md");

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

    #[test]
    fn install_blocks_same_name_real_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_dir.clone())]);

        let existing_dir = target_dir.join("brainstorming");
        fs::create_dir_all(&existing_dir).expect("create existing dir");
        fs::write(existing_dir.join("existing.txt"), "existing").expect("write existing file");

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        let error = install_skill(&mut config, "target-a", "brainstorming", &skills)
            .expect_err("existing dir should conflict");

        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(existing_dir.join("existing.txt").is_file());
        assert!(config.installations.is_empty());
    }

    #[test]
    fn install_blocks_same_name_regular_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_dir.clone())]);

        let existing_file = target_dir.join("brainstorming");
        fs::write(&existing_file, "existing file content").expect("write existing file");

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        let error = install_skill(&mut config, "target-a", "brainstorming", &skills)
            .expect_err("existing file should conflict");

        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(existing_file.is_file());
        assert_eq!(fs::read_to_string(&existing_file).unwrap(), "existing file content");
        assert!(config.installations.is_empty());
    }

    #[test]
    fn install_blocks_unknown_same_name_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_dir.clone())]);

        let other_source = temp.path().join("other-source");
        fs::create_dir_all(&other_source).expect("create other source");
        let unknown_link = target_dir.join("brainstorming");
        fs_adapter::create_dir_link(&other_source, &unknown_link, fs_adapter::default_link_type())
            .expect("create unknown link");

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        let error = install_skill(&mut config, "target-a", "brainstorming", &skills)
            .expect_err("unknown link should conflict");

        assert!(matches!(error, AppError::Conflict { .. }));
        assert!(fs_adapter::path_exists(&unknown_link));
        assert!(config.installations.is_empty());
    }

    #[test]
    fn uninstall_removes_only_recorded_link_and_preserves_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_dir.clone())]);

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install");

        let link_path = target_dir.join("brainstorming");
        assert!(fs_adapter::path_exists(&link_path));

        uninstall_skill(&mut config, "target-a", "brainstorming").expect("uninstall");

        assert!(!fs_adapter::path_exists(&link_path));
        assert!(config.installations.is_empty());
        assert!(skill.path.is_dir());
        assert!(skill.path.join("SKILL.md").is_file());
    }

    #[test]
    fn uninstall_refuses_missing_link_and_preserves_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_dir.clone())]);

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install");

        let link_path = target_dir.join("brainstorming");
        fs_adapter::remove_recorded_link(&link_path, &skill.path).expect("remove link externally");
        assert!(!fs_adapter::path_exists(&link_path));

        let error = uninstall_skill(&mut config, "target-a", "brainstorming").expect_err("missing link should fail");
        assert!(matches!(error, AppError::Io { .. }));
        assert_eq!(config.installations.len(), 1);
    }

    #[test]
    fn uninstall_refuses_mismatched_link_and_preserves_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_dir.clone())]);

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install");

        let link_path = target_dir.join("brainstorming");
        fs_adapter::remove_recorded_link(&link_path, &skill.path).expect("remove original link");
        let other_source = temp.path().join("other-source");
        fs::create_dir_all(&other_source).expect("create other source");
        fs_adapter::create_dir_link(&other_source, &link_path, fs_adapter::default_link_type())
            .expect("create mismatch link");

        let error = uninstall_skill(&mut config, "target-a", "brainstorming").expect_err("mismatch link should fail");
        assert!(matches!(error, AppError::Io { .. }));
        assert_eq!(config.installations.len(), 1);
    }

    #[test]
    fn main_skill_deletion_cleans_multiple_recorded_links() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_a_dir = create_target_dir(temp.path(), "a");
        let target_b_dir = create_target_dir(temp.path(), "b");

        let mut config = build_config(
            Some(&main_dir),
            &[
                ("target-a".to_string(), target_a_dir.clone()),
                ("target-b".to_string(), target_b_dir.clone()),
            ],
        );

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install a");
        install_skill(&mut config, "target-b", "brainstorming", &skills).expect("install b");

        let link_a = target_a_dir.join("brainstorming");
        let link_b = target_b_dir.join("brainstorming");

        let result = delete_main_skill(&mut config, "brainstorming", true).expect("delete main skill");

        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert_eq!(result.removed_link_count, 2);
        assert!(!skill.path.exists());
        assert!(!fs_adapter::path_exists(&link_a));
        assert!(!fs_adapter::path_exists(&link_b));
        assert!(config.installations.is_empty());
    }

    #[test]
    fn main_skill_deletion_aborts_when_recorded_link_cleanup_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_a_dir = create_target_dir(temp.path(), "a");
        let target_b_dir = create_target_dir(temp.path(), "b");

        let mut config = build_config(
            Some(&main_dir),
            &[
                ("target-a".to_string(), target_a_dir.clone()),
                ("target-b".to_string(), target_b_dir.clone()),
            ],
        );

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install a");
        install_skill(&mut config, "target-b", "brainstorming", &skills).expect("install b");

        let link_b = target_b_dir.join("brainstorming");

        // Corrupt link_b by replacing with a real directory
        fs_adapter::remove_recorded_link(&link_b, &skill.path).expect("remove link_b");
        fs::create_dir_all(&link_b).expect("create real dir at link_b");
        fs::write(link_b.join("keep.txt"), "keep").expect("write file");

        let error = delete_main_skill(&mut config, "brainstorming", true)
            .expect_err("should fail when link cleanup fails");
        assert!(matches!(error, AppError::Io { .. }));

        // Source skill should still exist
        assert!(skill.path.exists());
        assert!(skill.path.join("SKILL.md").is_file());

        // Both installation records should remain
        assert_eq!(config.installations.len(), 2);
    }

    #[test]
    fn main_skill_deletion_removes_related_installation_records_after_success() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let skill = create_valid_skill(&main_dir, "brainstorming");

        let target_a_dir = create_target_dir(temp.path(), "a");
        let mut config = build_config(Some(&main_dir), &[("target-a".to_string(), target_a_dir.clone())]);

        let skills = list_skills(Some(&main_dir)).expect("list skills");
        install_skill(&mut config, "target-a", "brainstorming", &skills).expect("install");

        // Add an unrelated installation record
        let unrelated_installation = Installation {
            id: "install-unrelated".to_string(),
            skill_dir_name: "other-skill".to_string(),
            skill_name: "other-skill".to_string(),
            source_path: main_dir.join("other-skill"),
            target_id: "target-a".to_string(),
            link_path: target_a_dir.join("other-skill"),
            link_type: fs_adapter::default_link_type(),
            created_at: "1".to_string(),
        };
        config.installations.push(unrelated_installation);

        let result = delete_main_skill(&mut config, "brainstorming", true).expect("delete main skill");

        assert_eq!(result.deleted_skill_dir_name, "brainstorming");
        assert_eq!(result.removed_link_count, 1);
        assert!(!skill.path.exists());

        // Only the unrelated installation record should remain
        assert_eq!(config.installations.len(), 1);
        assert_eq!(config.installations[0].skill_dir_name, "other-skill");
    }
}
