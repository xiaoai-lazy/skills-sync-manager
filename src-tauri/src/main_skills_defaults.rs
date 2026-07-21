use crate::agent_presets::{home_dir, normalize_platform_path};
use crate::models::{AppConfig, AppError};
use std::fs;
use std::path::{Path, PathBuf};

/// Default main skills library: `~/.skills-sync/skills`.
pub fn default_main_skills_dir_from_home(home: &Path) -> PathBuf {
    normalize_platform_path(home.join(".skills-sync").join("skills"))
}

/// When `main_skills_dir` is unset, create `~/.skills-sync/skills` and persist it.
/// Does not overwrite an already-configured path. Soft-skips if home is unknown
/// or the directory cannot be created (returns `Ok(false)`).
pub fn ensure_default_main_skills_dir(config: &mut AppConfig) -> Result<bool, AppError> {
    ensure_default_main_skills_dir_with_home(config, home_dir())
}

fn ensure_default_main_skills_dir_with_home(
    config: &mut AppConfig,
    home: Option<PathBuf>,
) -> Result<bool, AppError> {
    if config.settings.main_skills_dir.is_some() {
        return Ok(false);
    }

    let Some(home) = home else {
        return Ok(false);
    };

    let path = default_main_skills_dir_from_home(&home);
    if fs::create_dir_all(&path).is_err() {
        return Ok(false);
    }

    config.settings.main_skills_dir = Some(path);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Settings;
    use tempfile::tempdir;

    #[test]
    fn default_path_joins_skills_sync_skills() {
        let home = PathBuf::from(if cfg!(windows) {
            r"C:\Users\demo"
        } else {
            "/home/demo"
        });
        let path = default_main_skills_dir_from_home(&home);
        assert!(path.ends_with(Path::new(".skills-sync").join("skills")));
    }

    #[test]
    fn ensure_sets_and_creates_when_unset() {
        let tmp = tempdir().unwrap();
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();

        let mut config = AppConfig {
            settings: Settings {
                main_skills_dir: None,
                ..Settings::default()
            },
            ..AppConfig::default()
        };

        let changed =
            ensure_default_main_skills_dir_with_home(&mut config, Some(home.clone())).unwrap();
        assert!(changed);
        let expected = default_main_skills_dir_from_home(&home);
        assert_eq!(
            config.settings.main_skills_dir.as_deref(),
            Some(expected.as_path())
        );
        assert!(expected.is_dir());

        let again = ensure_default_main_skills_dir_with_home(&mut config, Some(home)).unwrap();
        assert!(!again);
    }

    #[test]
    fn ensure_preserves_existing_main_dir() {
        let existing = PathBuf::from(if cfg!(windows) {
            r"D:\my-skills"
        } else {
            "/opt/my-skills"
        });
        let mut config = AppConfig {
            settings: Settings {
                main_skills_dir: Some(existing.clone()),
                ..Settings::default()
            },
            ..AppConfig::default()
        };
        let changed = ensure_default_main_skills_dir_with_home(
            &mut config,
            Some(PathBuf::from("/tmp/unused-home")),
        )
        .unwrap();
        assert!(!changed);
        assert_eq!(config.settings.main_skills_dir, Some(existing));
    }

    #[test]
    fn ensure_skips_when_home_unknown() {
        let mut config = AppConfig {
            settings: Settings {
                main_skills_dir: None,
                ..Settings::default()
            },
            ..AppConfig::default()
        };
        let changed = ensure_default_main_skills_dir_with_home(&mut config, None).unwrap();
        assert!(!changed);
        assert!(config.settings.main_skills_dir.is_none());
    }
}
