use crate::models::{migrate_config, AppConfig, AppError, CURRENT_CONFIG_VERSION};
use crate::skill_migration::{self, MigrationReport};
use crate::skill_repos;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

static CONFIG_IO_LOCK: Mutex<()> = Mutex::new(());
static LAST_MIGRATION_REPORT: Mutex<Option<MigrationReport>> = Mutex::new(None);

pub fn take_last_migration_report() -> Option<MigrationReport> {
    match LAST_MIGRATION_REPORT.lock() {
        Ok(mut guard) => guard.take(),
        Err(_) => None,
    }
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    config_path: PathBuf,
}

impl ConfigStore {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    pub fn load(&self) -> Result<AppConfig, AppError> {
        let _guard = lock_config_io()?;
        self.load_unlocked()
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), AppError> {
        let _guard = lock_config_io()?;
        self.save_unlocked(config)
    }

    fn load_unlocked(&self) -> Result<AppConfig, AppError> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }

        let raw = fs::read_to_string(&self.config_path).map_err(|err| AppError::ConfigRead {
            path: self.config_path.clone(),
            message: err.to_string(),
        })?;

        let mut config: AppConfig = serde_json::from_str(&raw).map_err(|err| AppError::ConfigParse {
            path: self.config_path.clone(),
            message: err.to_string(),
        })?;

        let mut changed = false;

        if config.version < 5 {
            backup_config_file(&self.config_path)?;
            if migrate_config(&mut config) {
                changed = true;
            }
        }

        if config.version == 5 && CURRENT_CONFIG_VERSION == 6 {
            if let Some(main_dir) = config.settings.main_skills_dir.clone() {
                let report =
                    skill_migration::migrate_v5_to_v6(&mut config, &main_dir, &self.config_path)?;
                if let Ok(mut guard) = LAST_MIGRATION_REPORT.lock() {
                    *guard = Some(report);
                }
                changed = true;
            } else {
                config.version = 6;
                changed = true;
            }
        } else if config.version < CURRENT_CONFIG_VERSION {
            if migrate_config(&mut config) {
                changed = true;
            }
        }

        // Drop installation records whose main-library source is gone.
        let purged = skill_migration::purge_installations_with_missing_source(&mut config);
        if purged > 0 {
            changed = true;
        }

        // Recreate missing target links (never deletes existing wrong links).
        if config.version >= 6
            && !config.installations.is_empty()
            && skill_migration::has_stale_installation_links(&config)
        {
            let (repaired, failures) =
                skill_migration::repair_stale_installation_links(&config);
            if repaired > 0 || !failures.is_empty() {
                let report = skill_migration::MigrationReport {
                    backed_up_config: PathBuf::new(),
                    backed_up_main: None,
                    succeeded: Vec::new(),
                    failed: failures,
                    orphan_locals: Vec::new(),
                    links_repaired: repaired,
                };
                if let Ok(mut guard) = LAST_MIGRATION_REPORT.lock() {
                    if guard.is_none() {
                        *guard = Some(report);
                    }
                }
            }
        }
        if skill_repos::dedupe_skill_repos(&mut config) {
            changed = true;
        }
        if crate::models::normalize_config_paths(&mut config) {
            changed = true;
        }
        if crate::credential_store::reconcile_gitlab_credential_hosts(&mut config) {
            changed = true;
        }
        if crate::storage_keys::reconcile_storage_keys(&mut config) {
            changed = true;
        }
        if let Some(app_data_dir) = self.config_path.parent() {
            if crate::runtime_cache::migrate_from_config(app_data_dir, &mut config)? {
                changed = true;
            }
        }
        if crate::main_skills_defaults::ensure_default_main_skills_dir(&mut config)? {
            changed = true;
        }
        if changed {
            self.save_unlocked(&config)?;
        }

        Ok(config)
    }

    fn save_unlocked(&self, config: &AppConfig) -> Result<(), AppError> {
        // Never persist discover/update caches into config.json.
        let mut to_save = config.clone();
        crate::runtime_cache::strip_from_config(&mut to_save);

        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|err| AppError::ConfigWrite {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }

        let tmp_path = self.config_path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(&to_save).map_err(|err| AppError::ConfigWrite {
            path: self.config_path.clone(),
            message: err.to_string(),
        })?;

        fs::write(&tmp_path, raw).map_err(|err| AppError::ConfigWrite {
            path: tmp_path.clone(),
            message: err.to_string(),
        })?;

        self.replace_with_temp(tmp_path)
    }

    fn replace_with_temp(&self, tmp_path: PathBuf) -> Result<(), AppError> {
        let backup_path = self.config_path.with_extension("json.bak");

        remove_file_best_effort(&backup_path)?;

        let had_existing_config = self.config_path.exists();
        if had_existing_config {
            fs::rename(&self.config_path, &backup_path).map_err(|err| AppError::ConfigWrite {
                path: self.config_path.clone(),
                message: err.to_string(),
            })?;
        }

        match fs::rename(&tmp_path, &self.config_path) {
            Ok(()) => {
                remove_file_best_effort(&backup_path)?;
                Ok(())
            }
            Err(rename_err) => {
                if had_existing_config && backup_path.exists() {
                    if let Err(restore_err) = fs::rename(&backup_path, &self.config_path) {
                        return Err(AppError::ConfigWrite {
                            path: self.config_path.clone(),
                            message: format!(
                                "failed to replace config: {}; failed to restore backup: {}",
                                rename_err, restore_err
                            ),
                        });
                    }
                }

                Err(AppError::ConfigWrite {
                    path: self.config_path.clone(),
                    message: rename_err.to_string(),
                })
            }
        }
    }
}

fn lock_config_io() -> Result<std::sync::MutexGuard<'static, ()>, AppError> {
    CONFIG_IO_LOCK.lock().map_err(|err| AppError::Io {
        path: None,
        message: format!("config store lock poisoned: {}", err),
    })
}

fn remove_file_best_effort(path: &std::path::Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AppError::ConfigWrite {
            path: path.to_path_buf(),
            message: err.to_string(),
        }),
    }
}

fn backup_config_file(config_path: &std::path::Path) -> Result<(), AppError> {
    let backup_path = config_path.with_extension("json.backup-v4");
    fs::copy(config_path, &backup_path)
        .map_err(|err| AppError::Io {
            path: Some(backup_path),
            message: format!("failed to backup config before migration: {}", err),
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AppConfig, AppError, CURRENT_CONFIG_VERSION, Installation, LinkStrategy, LinkType, Target,
    };

    #[test]
    fn missing_config_returns_default() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ConfigStore::new(temp.path().join("config.json"));

        let config = store.load().expect("load default config");

        assert_eq!(config.version, CURRENT_CONFIG_VERSION);
        assert!(config.settings.main_skills_dir.is_none());
        assert_eq!(config.settings.link_strategy, LinkStrategy::Auto);
        assert!(config.targets.is_empty());
        assert!(config.installations.is_empty());
    }

    #[test]
    fn save_then_load_round_trips_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ConfigStore::new(temp.path().join("config.json"));
        let source_path = temp.path().join("main-skills").join("example-skill");
        let link_path = temp.path().join("target-skills").join("example-skill");
        fs::create_dir_all(&source_path).expect("create source skill dir");
        fs::create_dir_all(temp.path().join("target-skills")).expect("create target skills dir");
        crate::fs_adapter::create_dir_link(
            &source_path,
            &link_path,
            crate::fs_adapter::default_link_type(),
        )
        .expect("create installation link");
        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(temp.path().join("main-skills"));
        config.targets.push(Target::global_custom(
            "target-1",
            "Target One",
            temp.path().join("target-skills"),
            "2026-06-23T00:00:00Z",
            "2026-06-23T00:00:00Z",
        ));
        config.installations.push(Installation {
            id: "install-1".to_string(),
            skill_dir_name: "example-skill".to_string(),
            skill_name: "Example Skill".to_string(),
            source_path: source_path.clone(),
            target_id: "target-1".to_string(),
            link_path: link_path.clone(),
            link_type: crate::fs_adapter::default_link_type(),
            created_at: "2026-06-23T00:00:00Z".to_string(),
            ..Default::default()
        });

        store.save(&config).expect("save config");
        let loaded = store.load().expect("load saved config");

        assert_eq!(
            loaded.installations[0].skill_storage_key,
            "example-skill"
        );
        assert_eq!(loaded.settings, config.settings);
        assert_eq!(loaded.targets, config.targets);
        assert!(!store.config_path.with_extension("json.tmp").exists());
        assert!(!store.config_path.with_extension("json.bak").exists());
    }

    #[test]
    fn load_purges_installations_whose_source_path_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ConfigStore::new(temp.path().join("config.json"));
        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(temp.path().join("main-skills"));
        config.installations.push(Installation {
            id: "install-1".to_string(),
            skill_dir_name: "gone-skill".to_string(),
            skill_name: "Gone Skill".to_string(),
            source_path: temp.path().join("main-skills").join("gone-skill"),
            target_id: "target-1".to_string(),
            link_path: temp.path().join("target-skills").join("gone-skill"),
            link_type: LinkType::Symlink,
            created_at: "2026-06-23T00:00:00Z".to_string(),
            ..Default::default()
        });

        store.save(&config).expect("save config");
        let loaded = store.load().expect("load saved config");

        assert!(loaded.installations.is_empty());
    }

    #[test]
    fn save_overwrites_existing_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ConfigStore::new(temp.path().join("config.json"));
        let mut first_config = AppConfig::default();
        first_config.settings.main_skills_dir = Some(temp.path().join("first-main-skills"));
        let mut second_config = AppConfig::default();
        second_config.settings.main_skills_dir = Some(temp.path().join("second-main-skills"));

        store.save(&first_config).expect("save first config");
        store
            .save(&second_config)
            .expect("overwrite with second config");
        let loaded = store.load().expect("load overwritten config");

        assert_eq!(
            loaded.settings.main_skills_dir,
            second_config.settings.main_skills_dir
        );
        assert!(!store.config_path.with_extension("json.tmp").exists());
        assert!(!store.config_path.with_extension("json.bak").exists());
    }

    #[test]
    fn failed_replacement_restores_existing_config_from_backup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.json");
        let store = ConfigStore::new(config_path.clone());
        let old_contents = "{\"version\":1,\"settings\":{\"mainSkillsDir\":null,\"linkStrategy\":\"auto\"},\"targets\":[],\"installations\":[]}";
        fs::write(&config_path, old_contents).expect("write existing config");
        let missing_tmp_path = temp.path().join("missing-config.json.tmp");

        let error = store
            .replace_with_temp(missing_tmp_path)
            .expect_err("missing temp file should fail replacement");

        assert!(matches!(error, AppError::ConfigWrite { .. }));
        assert_eq!(fs::read_to_string(&config_path).unwrap(), old_contents);
        assert!(!store.config_path.with_extension("json.bak").exists());
    }

    #[test]
    fn load_v0_2_config_migrates_and_persists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.json");
        let old = r#"{"version":1,"settings":{"mainSkillsDir":null,"linkStrategy":"auto"},"targets":[],"installations":[]}"#;
        fs::write(&path, old).expect("write v0.2 config");
        let store = ConfigStore::new(path.clone());

        let config = store.load().expect("load should migrate v0.2 config");

        assert_eq!(config.version, crate::models::CURRENT_CONFIG_VERSION);
        assert!(config.skill_repos.is_empty());
        assert!(config.skill_records.is_empty());

        let on_disk = fs::read_to_string(path).expect("read migrated config");
        assert!(on_disk.contains("skillRepos"));
        assert!(on_disk.contains("\"version\": 6"));
    }

    #[test]
    fn current_config_does_not_rewrite_on_load() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("skills");
        fs::create_dir_all(&main_dir).expect("create main skills dir");

        let store = ConfigStore::new(temp.path().join("config.json"));
        let mut config = AppConfig::default();
        // Already configured — load must not rewrite (including default main-dir ensure).
        config.settings.main_skills_dir =
            Some(crate::agent_presets::normalize_platform_path(&main_dir));
        store.save(&config).expect("save config");

        let before = fs::read_to_string(store.config_path.clone()).expect("read config");
        let loaded = store.load().expect("reload config");
        let after = fs::read_to_string(store.config_path.clone()).expect("read config again");

        assert_eq!(loaded, config);
        assert_eq!(before, after);
    }

    #[test]
    fn load_sets_default_main_skills_dir_when_unset() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ConfigStore::new(temp.path().join("config.json"));
        let config = AppConfig::default();
        assert!(config.settings.main_skills_dir.is_none());
        store.save(&config).expect("save config");

        let loaded = store.load().expect("load should set default main dir");
        assert!(loaded.settings.main_skills_dir.is_some());
        let path = loaded.settings.main_skills_dir.as_ref().unwrap();
        assert!(path.ends_with(std::path::Path::new(".skills-sync").join("skills")));
        assert!(path.is_dir());
    }

    #[test]
    fn malformed_config_returns_error_without_overwrite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.json");
        fs::write(&path, "{not json").expect("write malformed config");
        let store = ConfigStore::new(path.clone());

        let error = store.load().expect_err("malformed config should fail");

        assert!(matches!(error, AppError::ConfigParse { .. }));
        assert_eq!(fs::read_to_string(path).unwrap(), "{not json");
    }

    #[test]
    fn default_config_includes_skill_hub_fields() {
        let config = AppConfig::default();
        assert!(config.skill_repos.is_empty());
        assert!(config.skill_records.is_empty());
        assert!(config.skill_discover_cache.skills.is_empty());
        assert!(config.skill_update_cache.updates.is_empty());
    }

    #[test]
    fn load_v4_creates_backup_before_migrate() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.json");
        let v4 = r#"{"version":4,"settings":{"mainSkillsDir":null,"linkStrategy":"auto"},"targets":[{"id":"t1","name":"X","skillsDir":"D:/skills","createdAt":"1","updatedAt":"1"}],"installations":[]}"#;
        fs::write(&path, v4).expect("write v4 config");
        let store = ConfigStore::new(path.clone());

        let config = store.load().expect("load should migrate v4 config");

        assert_eq!(config.version, CURRENT_CONFIG_VERSION);
        let backup = temp.path().join("config.json.backup-v4");
        assert!(backup.exists(), "backup file should exist before migration write");
        assert_eq!(fs::read_to_string(&backup).unwrap(), v4);
    }

    #[test]
    fn backup_failure_aborts_migration_without_writing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.json");
        let v4 = r#"{"version":4,"settings":{"mainSkillsDir":null,"linkStrategy":"auto"},"targets":[],"installations":[]}"#;
        fs::write(&path, v4).expect("write v4 config");
        fs::create_dir_all(temp.path().join("config.json.backup-v4")).expect("block backup");
        let store = ConfigStore::new(path.clone());

        let error = store.load().expect_err("backup failure should abort migration");

        assert!(matches!(
            error,
            AppError::Io { .. } | AppError::ConfigWrite { .. }
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), v4);
    }

    #[test]
    fn concurrent_saves_do_not_corrupt_config() {
        use std::sync::Arc;
        use std::thread;

        let temp = tempfile::tempdir().expect("tempdir");
        let temp_dir = temp.path().to_path_buf();
        let store = Arc::new(ConfigStore::new(temp_dir.join("config.json")));
        store
            .save(&AppConfig::default())
            .expect("seed config");

        let mut handles = Vec::new();
        for index in 0..8 {
            let store = Arc::clone(&store);
            let temp_dir = temp_dir.clone();
            handles.push(thread::spawn(move || {
                let mut config = store.load().expect("load config");
                config.settings.main_skills_dir = Some(temp_dir.join(format!("main-{index}")));
                store.save(&config).expect("save config");
            }));
        }

        for handle in handles {
            handle.join().expect("thread should finish");
        }

        let loaded = store.load().expect("load final config");
        assert!(loaded.settings.main_skills_dir.is_some());
        assert!(store.config_path.exists());
        assert!(!store.config_path.with_extension("json.bak").exists());
    }

    #[test]
    fn save_creates_parent_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp
            .path()
            .join("nested")
            .join("settings")
            .join("config.json");
        let store = ConfigStore::new(config_path.clone());

        store
            .save(&AppConfig::default())
            .expect("save creates parent directories");

        assert!(config_path.exists());
        assert!(config_path.parent().unwrap().is_dir());
        assert!(!config_path.with_extension("json.tmp").exists());
        assert!(!config_path.with_extension("json.bak").exists());
    }

    #[test]
    fn take_last_migration_report_after_v5_load() {
        use crate::models::SkillRecord;

        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let flat = main.join("tdd");
        fs::create_dir_all(&flat).expect("create flat skill");
        fs::write(
            flat.join("SKILL.md"),
            "---\nname: tdd\ndescription: Test.\n---\n\n# TDD\n",
        )
        .expect("write skill");

        let config_path = temp.path().join("config.json");
        let mut config = AppConfig::default();
        config.version = 5;
        config.settings.main_skills_dir = Some(main.clone());
        config.skill_records.insert(
            "tdd".to_string(),
            SkillRecord {
                source: "github".to_string(),
                repo_host: "github.com".to_string(),
                project_path: "anthropics/skills".to_string(),
                repo_owner: "anthropics".to_string(),
                repo_name: "skills".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/tdd".to_string(),
                content_hash: "abc".to_string(),
                installed_at: "2026-01-01".to_string(),
                ..Default::default()
            },
        );
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).expect("serialize v5 config"),
        )
        .expect("write v5 config");

        let store = ConfigStore::new(config_path.clone());
        let loaded = store.load().expect("load should migrate v5 config");

        assert_eq!(loaded.version, CURRENT_CONFIG_VERSION);
        assert!(config_path.with_extension("json.backup-v5").exists());
        assert!(temp.path().join("migration-v5-v6.log").exists());

        let report = take_last_migration_report().expect("report should be available once");
        assert_eq!(report.succeeded, vec!["repo/github.com--anthropics-skills/tdd"]);
        assert!(report.failed.is_empty());
        assert!(take_last_migration_report().is_none());
    }
}
