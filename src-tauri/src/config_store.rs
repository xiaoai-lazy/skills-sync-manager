use crate::models::{migrate_config, AppConfig, AppError};
use crate::skill_repos;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigStore {
    config_path: PathBuf,
}

impl ConfigStore {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    pub fn load(&self) -> Result<AppConfig, AppError> {
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
        if migrate_config(&mut config) {
            changed = true;
        }
        if skill_repos::ensure_builtin_repos(&mut config) {
            changed = true;
        }
        if skill_repos::dedupe_skill_repos(&mut config) {
            changed = true;
        }
        if changed {
            self.save(&config)?;
        }

        Ok(config)
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), AppError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|err| AppError::ConfigWrite {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }

        let tmp_path = self.config_path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(config).map_err(|err| AppError::ConfigWrite {
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

        if backup_path.exists() {
            fs::remove_file(&backup_path).map_err(|err| AppError::ConfigWrite {
                path: backup_path.clone(),
                message: err.to_string(),
            })?;
        }

        let had_existing_config = self.config_path.exists();
        if had_existing_config {
            fs::rename(&self.config_path, &backup_path).map_err(|err| AppError::ConfigWrite {
                path: self.config_path.clone(),
                message: err.to_string(),
            })?;
        }

        match fs::rename(&tmp_path, &self.config_path) {
            Ok(()) => {
                if had_existing_config && backup_path.exists() {
                    fs::remove_file(&backup_path).map_err(|err| AppError::ConfigWrite {
                        path: backup_path,
                        message: err.to_string(),
                    })?;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{migrate_config, AppConfig, AppError, CURRENT_CONFIG_VERSION, Installation, LinkStrategy, LinkType, Target};

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
        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(temp.path().join("main-skills"));
        config.targets.push(Target {
            id: "target-1".to_string(),
            name: "Target One".to_string(),
            skills_dir: temp.path().join("target-skills"),
            created_at: "2026-06-23T00:00:00Z".to_string(),
            updated_at: "2026-06-23T00:00:00Z".to_string(),
        });
        config.installations.push(Installation {
            id: "install-1".to_string(),
            skill_dir_name: "example-skill".to_string(),
            skill_name: "Example Skill".to_string(),
            source_path: temp.path().join("main-skills").join("example-skill"),
            target_id: "target-1".to_string(),
            link_path: temp.path().join("target-skills").join("example-skill"),
            link_type: LinkType::Symlink,
            created_at: "2026-06-23T00:00:00Z".to_string(),
        });

        store.save(&config).expect("save config");
        let loaded = store.load().expect("load saved config");

        assert_eq!(loaded, config);
        assert!(!store.config_path.with_extension("json.tmp").exists());
        assert!(!store.config_path.with_extension("json.bak").exists());
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
        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].owner, "obra");
        assert_eq!(config.skill_repos[0].name, "superpowers");
        assert!(config.skill_records.is_empty());

        let on_disk = fs::read_to_string(path).expect("read migrated config");
        assert!(on_disk.contains("skillRepos"));
        assert!(on_disk.contains("\"version\": 4"));
    }

    #[test]
    fn current_config_does_not_rewrite_on_load() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ConfigStore::new(temp.path().join("config.json"));
        let config = AppConfig::default();
        store.save(&config).expect("save config");

        let before = fs::read_to_string(store.config_path.clone()).expect("read config");
        let loaded = store.load().expect("reload config");
        let after = fs::read_to_string(store.config_path.clone()).expect("read config again");

        assert_eq!(loaded, config);
        assert_eq!(before, after);
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
        assert_eq!(config.skill_repos.len(), 1);
        assert_eq!(config.skill_repos[0].owner, "obra");
        assert_eq!(config.skill_repos[0].name, "superpowers");
        assert!(config.skill_records.is_empty());
        assert!(config.skill_discover_cache.skills.is_empty());
        assert!(config.skill_update_cache.updates.is_empty());
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
}
