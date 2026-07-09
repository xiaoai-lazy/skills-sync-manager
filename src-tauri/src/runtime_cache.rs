use crate::models::{AppConfig, AppError, SkillDiscoverCache, SkillUpdateCache};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCache {
    #[serde(default = "default_runtime_cache_version")]
    pub version: u32,
    #[serde(default)]
    pub skill_discover_cache: SkillDiscoverCache,
    #[serde(default)]
    pub skill_update_cache: SkillUpdateCache,
}

fn default_runtime_cache_version() -> u32 {
    1
}

impl Default for RuntimeCache {
    fn default() -> Self {
        Self {
            version: 1,
            skill_discover_cache: SkillDiscoverCache::default(),
            skill_update_cache: SkillUpdateCache::default(),
        }
    }
}

pub fn runtime_cache_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("runtime-cache.json")
}

pub fn load(app_data_dir: &Path) -> RuntimeCache {
    let path = runtime_cache_path(app_data_dir);
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => RuntimeCache::default(),
    }
}

pub fn save(app_data_dir: &Path, cache: &RuntimeCache) -> Result<(), AppError> {
    let path = runtime_cache_path(app_data_dir);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| AppError::ConfigWrite {
            path: parent.to_path_buf(),
            message: err.to_string(),
        })?;
    }

    let tmp_path = path.with_extension("json.tmp");
    let raw = serde_json::to_string_pretty(cache).map_err(|err| AppError::ConfigWrite {
        path: path.clone(),
        message: err.to_string(),
    })?;

    fs::write(&tmp_path, raw).map_err(|err| AppError::ConfigWrite {
        path: tmp_path.clone(),
        message: err.to_string(),
    })?;

    replace_with_temp(&path, tmp_path)
}

fn replace_with_temp(final_path: &Path, tmp_path: PathBuf) -> Result<(), AppError> {
    if final_path.exists() {
        fs::remove_file(final_path).map_err(|err| AppError::ConfigWrite {
            path: final_path.to_path_buf(),
            message: err.to_string(),
        })?;
    }

    fs::rename(&tmp_path, final_path).map_err(|err| AppError::ConfigWrite {
        path: final_path.to_path_buf(),
        message: err.to_string(),
    })?;

    Ok(())
}

/// If config still holds discover/update caches, overwrite runtime-cache and clear
/// those fields on config. Returns true when config was changed.
pub fn migrate_from_config(app_data_dir: &Path, config: &mut AppConfig) -> Result<bool, AppError> {
    let has_discover = !config.skill_discover_cache.skills.is_empty()
        || config.skill_discover_cache.fetched_at.is_some();
    let has_updates = !config.skill_update_cache.updates.is_empty()
        || config.skill_update_cache.checked_at.is_some();
    if !has_discover && !has_updates {
        return Ok(false);
    }
    let cache = RuntimeCache {
        version: 1,
        skill_discover_cache: std::mem::take(&mut config.skill_discover_cache),
        skill_update_cache: std::mem::take(&mut config.skill_update_cache),
    };
    save(app_data_dir, &cache)?;
    Ok(true)
}

pub fn strip_from_config(config: &mut AppConfig) {
    config.skill_discover_cache = SkillDiscoverCache::default();
    config.skill_update_cache = SkillUpdateCache::default();
}

pub fn attach_to_config(app_data_dir: &Path, config: &mut AppConfig) {
    let cache = load(app_data_dir);
    config.skill_discover_cache = cache.skill_discover_cache;
    config.skill_update_cache = cache.skill_update_cache;
}

pub fn persist_from_config(app_data_dir: &Path, config: &AppConfig) -> Result<(), AppError> {
    save(
        app_data_dir,
        &RuntimeCache {
            version: 1,
            skill_discover_cache: config.skill_discover_cache.clone(),
            skill_update_cache: config.skill_update_cache.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DiscoverableSkill, SkillDiscoverCache, SkillUpdateCache};

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let cache = load(dir.path());
        assert_eq!(cache, RuntimeCache::default());
    }

    #[test]
    fn load_corrupt_json_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(runtime_cache_path(dir.path()), "{not-json").unwrap();
        let cache = load(dir.path());
        assert_eq!(cache, RuntimeCache::default());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = RuntimeCache::default();
        cache.skill_discover_cache.fetched_at = Some("2026-07-09T00:00:00Z".into());
        save(dir.path(), &cache).unwrap();
        assert_eq!(load(dir.path()), cache);
        assert!(!runtime_cache_path(dir.path()).with_extension("json.tmp").exists());
    }

    #[test]
    fn migrate_from_config_copies_caches_and_clears_config_fields() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.skill_discover_cache = SkillDiscoverCache {
            fetched_at: Some("t1".into()),
            skills: vec![DiscoverableSkill::default()],
        };
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("t2".into()),
            updates: vec![],
        };

        let changed = migrate_from_config(dir.path(), &mut config).unwrap();
        assert!(changed);
        assert!(config.skill_discover_cache.skills.is_empty());
        assert!(config.skill_discover_cache.fetched_at.is_none());
        assert!(config.skill_update_cache.updates.is_empty());
        assert!(config.skill_update_cache.checked_at.is_none());
        let loaded = load(dir.path());
        assert_eq!(loaded.skill_discover_cache.fetched_at.as_deref(), Some("t1"));
        assert_eq!(loaded.skill_update_cache.checked_at.as_deref(), Some("t2"));
        assert_eq!(loaded.skill_discover_cache.skills.len(), 1);
    }

    #[test]
    fn migrate_from_config_noop_when_caches_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        let changed = migrate_from_config(dir.path(), &mut config).unwrap();
        assert!(!changed);
        assert!(!runtime_cache_path(dir.path()).exists());
    }

    #[test]
    fn save_strips_caches_from_config_json_but_attach_restores_them() {
        use crate::config_store::ConfigStore;

        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let store = ConfigStore::new(config_path.clone());

        let mut config = AppConfig::default();
        config.skill_discover_cache = SkillDiscoverCache {
            fetched_at: Some("t-discover".into()),
            skills: vec![DiscoverableSkill::default()],
        };
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("t-update".into()),
            updates: vec![],
        };
        persist_from_config(dir.path(), &config).unwrap();
        store.save(&config).unwrap();

        let raw = fs::read_to_string(&config_path).unwrap();
        let disk: AppConfig = serde_json::from_str(&raw).unwrap();
        assert!(disk.skill_discover_cache.skills.is_empty());
        assert!(disk.skill_update_cache.checked_at.is_none());

        let mut loaded = store.load().unwrap();
        assert!(loaded.skill_discover_cache.skills.is_empty());
        attach_to_config(dir.path(), &mut loaded);
        assert_eq!(
            loaded.skill_discover_cache.fetched_at.as_deref(),
            Some("t-discover")
        );
        assert_eq!(loaded.skill_discover_cache.skills.len(), 1);
    }
}
