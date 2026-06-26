use crate::models::{AppConfig, AppError, AppErrorDto, AppState};
use std::path::PathBuf;
use tauri::Manager;

fn store_from_app(app: &tauri::AppHandle) -> Result<crate::config_store::ConfigStore, AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|err| AppError::Io {
        path: None,
        message: format!("failed to resolve app data directory: {}", err),
    })?;
    let config_path = app_data_dir.join("config.json");
    Ok(crate::config_store::ConfigStore::new(config_path))
}

pub fn build_app_state(
    config: AppConfig,
    selected_target_id: Option<String>,
) -> Result<AppState, AppError> {
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())?;
    let selected = selected_target_id
        .and_then(|id| config.targets.iter().find(|t| t.id == id).map(|_| id))
        .or_else(|| config.targets.first().map(|t| t.id.clone()));

    let selected_target_skills = match selected.as_deref() {
        Some(target_id) => crate::link_installer::compute_target_skill_states(
            &config,
            target_id,
            &skills,
        )?,
        None => Vec::new(),
    };

    Ok(AppState {
        config,
        skills,
        selected_target_id: selected,
        selected_target_skills,
    })
}

fn run_with_config<F>(
    app: tauri::AppHandle,
    mutate: F,
    selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto>
where
    F: FnOnce(&mut AppConfig) -> Result<(), AppError>,
{
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    mutate(&mut config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    build_app_state(config, selected_target_id).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn get_app_state(
    app: tauri::AppHandle,
    selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    build_app_state(config, selected_target_id).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn set_main_skills_dir(
    app: tauri::AppHandle,
    path: String,
) -> Result<AppState, AppErrorDto> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(AppError::InvalidMainSkillsDir {
            path: path.clone(),
            message: "Path does not exist".to_string(),
        }.to_dto());
    }
    if !path.is_dir() {
        return Err(AppError::InvalidMainSkillsDir {
            path: path.clone(),
            message: "Path is not a directory".to_string(),
        }.to_dto());
    }

    run_with_config(
        app,
        |config| {
            config.settings.main_skills_dir = Some(path.clone());
            Ok(())
        },
        None,
    )
}

#[tauri::command]
pub fn add_target(
    app: tauri::AppHandle,
    name: String,
    skills_dir: String,
) -> Result<AppState, AppErrorDto> {
    let skills_dir = PathBuf::from(skills_dir);
    let request = crate::target_registry::AddTargetRequest {
        name,
        skills_dir,
    };

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let target = crate::target_registry::add_target(&mut config, request).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    build_app_state(config, Some(target.id)).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn update_target(
    app: tauri::AppHandle,
    target_id: String,
    name: String,
    skills_dir: String,
) -> Result<AppState, AppErrorDto> {
    let skills_dir = PathBuf::from(skills_dir);
    let request = crate::target_registry::UpdateTargetRequest {
        name,
        skills_dir,
    };
    let target_id_for_closure = target_id.clone();

    run_with_config(
        app,
        |config| {
            crate::target_registry::update_target(config, &target_id_for_closure, request.clone())?;
            Ok(())
        },
        Some(target_id),
    )
}

#[tauri::command]
pub fn delete_target(
    app: tauri::AppHandle,
    target_id: String,
    cleanup_recorded_links: bool,
) -> Result<AppState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let installation_count = config
        .installations
        .iter()
        .filter(|i| i.target_id == target_id)
        .count();

    if installation_count > 0 && !cleanup_recorded_links {
        return Err(AppError::TargetHasInstallations {
            target_id: target_id.clone(),
            installation_count,
        }.to_dto());
    }

    if cleanup_recorded_links {
        let to_uninstall: Vec<String> = config
            .installations
            .iter()
            .filter(|i| i.target_id == target_id)
            .map(|i| i.skill_dir_name.clone())
            .collect();
        for skill_dir_name in to_uninstall {
            crate::link_installer::uninstall_skill(&mut config, &target_id, &skill_dir_name)
                .map_err(|err| err.to_dto())?;
        }
    }

    crate::target_registry::delete_target_config(&mut config, &target_id)
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    build_app_state(config, None).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn install_skill(
    app: tauri::AppHandle,
    target_id: String,
    skill_dir_name: String,
) -> Result<AppState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    crate::link_installer::install_skill(&mut config, &target_id, &skill_dir_name, &skills)
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    build_app_state(config, Some(target_id)).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn uninstall_skill(
    app: tauri::AppHandle,
    target_id: String,
    skill_dir_name: String,
) -> Result<AppState, AppErrorDto> {
    let target_id_for_closure = target_id.clone();
    let skill_dir_name_for_closure = skill_dir_name.clone();

    run_with_config(
        app,
        |config| {
            crate::link_installer::uninstall_skill(config, &target_id_for_closure, &skill_dir_name_for_closure)?;
            Ok(())
        },
        Some(target_id),
    )
}

#[tauri::command]
pub fn delete_main_skill(
    app: tauri::AppHandle,
    skill_dir_name: String,
    confirmed: bool,
) -> Result<AppState, AppErrorDto> {
    let skill_dir_name_for_closure = skill_dir_name.clone();

    run_with_config(
        app,
        |config| {
            crate::skill_remover::delete_main_skill(config, &skill_dir_name_for_closure, confirmed)?;
            Ok(())
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Settings, SkillView, Target};
    use std::fs;

    fn create_config_with_targets(main_dir: Option<PathBuf>, targets: Vec<Target>) -> AppConfig {
        AppConfig {
            version: 1,
            settings: Settings {
                main_skills_dir: main_dir,
                link_strategy: crate::models::LinkStrategy::Auto,
            },
            targets,
            installations: Vec::new(),
        }
    }

    fn create_target(id: &str, skills_dir: PathBuf) -> Target {
        Target {
            id: id.to_string(),
            name: format!("Target {}", id),
            skills_dir,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    fn create_valid_skill(main_dir: &std::path::Path, dir_name: &str) -> SkillView {
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

    #[test]
    fn build_app_state_with_no_targets_returns_empty_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let config = create_config_with_targets(Some(main_dir.clone()), Vec::new());
        let state = build_app_state(config, None).expect("build state");

        assert_eq!(state.skills.len(), 1);
        assert_eq!(state.skills[0].dir_name, "brainstorming");
        assert!(state.selected_target_id.is_none());
        assert!(state.selected_target_skills.is_empty());
    }

    #[test]
    fn build_app_state_selects_first_target_by_default() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let target_dir = temp.path().join("target-skills");
        fs::create_dir_all(&target_dir).expect("create target dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let targets = vec![create_target("target-1", target_dir.clone())];
        let config = create_config_with_targets(Some(main_dir.clone()), targets);
        let state = build_app_state(config, None).expect("build state");

        assert_eq!(state.selected_target_id, Some("target-1".to_string()));
        assert_eq!(state.selected_target_skills.len(), 1);
        assert_eq!(state.selected_target_skills[0].skill.dir_name, "brainstorming");
    }

    #[test]
    fn build_app_state_respects_explicit_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let target1_dir = temp.path().join("target-1");
        fs::create_dir_all(&target1_dir).expect("create target1 dir");
        let target2_dir = temp.path().join("target-2");
        fs::create_dir_all(&target2_dir).expect("create target2 dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let targets = vec![
            create_target("target-1", target1_dir.clone()),
            create_target("target-2", target2_dir.clone()),
        ];
        let config = create_config_with_targets(Some(main_dir.clone()), targets);
        let state = build_app_state(config, Some("target-2".to_string())).expect("build state");

        assert_eq!(state.selected_target_id, Some("target-2".to_string()));
    }

    #[test]
    fn build_app_state_falls_back_to_first_when_explicit_selection_is_invalid() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let target_dir = temp.path().join("target-skills");
        fs::create_dir_all(&target_dir).expect("create target dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let targets = vec![create_target("target-1", target_dir.clone())];
        let config = create_config_with_targets(Some(main_dir.clone()), targets);
        let state = build_app_state(config, Some("nonexistent".to_string())).expect("build state");

        assert_eq!(state.selected_target_id, Some("target-1".to_string()));
    }

    #[test]
    fn build_app_state_returns_error_for_invalid_main_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing_dir = temp.path().join("missing");

        let config = create_config_with_targets(Some(missing_dir.clone()), Vec::new());
        let error = build_app_state(config, None).expect_err("should fail");

        assert!(matches!(error, AppError::InvalidMainSkillsDir { .. }));
    }

    #[test]
    fn build_app_state_with_no_main_dir_returns_empty_skills() {
        let config = create_config_with_targets(None, Vec::new());
        let state = build_app_state(config, None).expect("build state");

        assert!(state.skills.is_empty());
        assert!(state.selected_target_skills.is_empty());
    }

    #[test]
    fn app_error_to_dto_maps_variants_correctly() {
        let conflict = AppError::Conflict {
            path: PathBuf::from("/some/path"),
            message: "already exists".to_string(),
        };
        let dto = conflict.to_dto();
        assert_eq!(dto.code, "conflict");
        assert!(dto.message.contains("/some/path"));
        assert!(dto.message.contains("already exists"));

        let target_not_found = AppError::TargetNotFound {
            target_id: "target-1".to_string(),
        };
        let dto = target_not_found.to_dto();
        assert_eq!(dto.code, "targetNotFound");
        assert!(dto.message.contains("target-1"));

        let invalid_target_name = AppError::InvalidTargetName;
        let dto = invalid_target_name.to_dto();
        assert_eq!(dto.code, "invalidTargetName");
        assert!(dto.message.contains("blank"));

        let target_has_installs = AppError::TargetHasInstallations {
            target_id: "target-1".to_string(),
            installation_count: 3,
        };
        let dto = target_has_installs.to_dto();
        assert_eq!(dto.code, "targetHasInstallations");
        assert!(dto.message.contains("3"));

        let io = AppError::Io {
            path: Some(PathBuf::from("/some/path")),
            message: "permission denied".to_string(),
        };
        let dto = io.to_dto();
        assert_eq!(dto.code, "io");
        assert!(dto.message.contains("permission denied"));

        let io_no_path = AppError::Io {
            path: None,
            message: "generic error".to_string(),
        };
        let dto = io_no_path.to_dto();
        assert_eq!(dto.code, "io");
        assert!(dto.message.contains("generic error"));
        assert!(!dto.message.contains("at "));
    }

    #[test]
    fn app_error_to_dto_maps_config_errors() {
        let config_read = AppError::ConfigRead {
            path: PathBuf::from("/config.json"),
            message: "not found".to_string(),
        };
        let dto = config_read.to_dto();
        assert_eq!(dto.code, "configRead");

        let config_parse = AppError::ConfigParse {
            path: PathBuf::from("/config.json"),
            message: "invalid json".to_string(),
        };
        let dto = config_parse.to_dto();
        assert_eq!(dto.code, "configParse");

        let config_write = AppError::ConfigWrite {
            path: PathBuf::from("/config.json"),
            message: "disk full".to_string(),
        };
        let dto = config_write.to_dto();
        assert_eq!(dto.code, "configWrite");
    }

    #[test]
    fn app_error_to_dto_maps_invalid_main_skills_dir() {
        let err = AppError::InvalidMainSkillsDir {
            path: PathBuf::from("/skills"),
            message: "not a directory".to_string(),
        };
        let dto = err.to_dto();
        assert_eq!(dto.code, "invalidMainSkillsDir");
        assert!(dto.message.contains("/skills"));
    }

    #[test]
    fn app_error_to_dto_maps_invalid_skill() {
        let err = AppError::InvalidSkill {
            skill_dir_name: "bad-skill".to_string(),
            message: "missing metadata".to_string(),
        };
        let dto = err.to_dto();
        assert_eq!(dto.code, "invalidSkill");
        assert!(dto.message.contains("bad-skill"));
    }

    #[test]
    fn app_error_to_dto_maps_invalid_target_dir() {
        let err = AppError::InvalidTargetDir {
            path: PathBuf::from("/target"),
            message: "not writable".to_string(),
        };
        let dto = err.to_dto();
        assert_eq!(dto.code, "invalidTargetDir");
        assert!(dto.message.contains("/target"));
    }
}
