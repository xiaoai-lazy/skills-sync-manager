use crate::agent_presets::{self, AgentPreset};
use crate::agent_presets::normalize_platform_path;
use crate::models::{
    AgentPresetDto, AppConfig, AppError, AppErrorDto, AppState, MigrationReportDto, SkillView,
    SyncTargetInstallationsResponse, TargetScope,
};
use std::path::{Path, PathBuf};
use tauri::Manager;

pub mod skill_hub;
pub mod updater;

pub(crate) fn store_from_app(app: &tauri::AppHandle) -> Result<crate::config_store::ConfigStore, AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|err| AppError::Io {
        path: None,
        message: format!("failed to resolve app data directory: {}", err),
    })?;
    let config_path = app_data_dir.join("config.json");
    Ok(crate::config_store::ConfigStore::new(config_path))
}

pub enum AppStateBuildMode {
    /// Call list_skills and return a full AppState.
    Full,
    /// Reuse a skills snapshot; do not call list_skills again.
    Light { skills: Vec<SkillView> },
}

pub fn build_app_state_with_mode(
    mut config: AppConfig,
    selected_target_id: Option<String>,
    app_data_dir: Option<&Path>,
    mode: AppStateBuildMode,
) -> Result<AppState, AppError> {
    if let Some(dir) = app_data_dir {
        crate::runtime_cache::attach_to_config(dir, &mut config);
    }

    let skills = match &mode {
        AppStateBuildMode::Full => {
            crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())?
        }
        AppStateBuildMode::Light { skills } => skills.clone(),
    };
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
        last_migration_report: None,
        skills_included: true,
        cleanup_warnings: Vec::new(),
    })
}

pub fn build_app_state(
    config: AppConfig,
    selected_target_id: Option<String>,
    app_data_dir: Option<&Path>,
) -> Result<AppState, AppError> {
    build_app_state_with_mode(
        config,
        selected_target_id,
        app_data_dir,
        AppStateBuildMode::Full,
    )
}

fn run_with_config<F>(
    app: tauri::AppHandle,
    mutate: F,
    selected_target_id: Option<String>,
    rescan_library: bool,
) -> Result<AppState, AppErrorDto>
where
    F: FnOnce(&mut AppConfig) -> Result<(), AppError>,
{
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let light_skills = if rescan_library {
        None
    } else {
        Some(
            crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
                .map_err(|err| err.to_dto())?,
        )
    };

    mutate(&mut config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    let mode = match light_skills {
        Some(skills) => AppStateBuildMode::Light { skills },
        None => AppStateBuildMode::Full,
    };
    build_app_state_with_mode(config, selected_target_id, Some(app_data_dir.as_path()), mode)
        .map_err(|err| err.to_dto())
}

fn app_data_dir_from_app(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    app.path().app_data_dir().map_err(|err| AppError::Io {
        path: None,
        message: format!("failed to resolve app data directory: {}", err),
    })
}

fn resource_dir_from_app(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().resource_dir().ok()
}

pub(crate) fn parse_target_scope(scope: &str) -> Result<TargetScope, AppError> {
    match scope.trim().to_ascii_lowercase().as_str() {
        "global" => Ok(TargetScope::Global),
        "project" => Ok(TargetScope::Project),
        other => Err(AppError::InvalidInput {
            input: scope.to_string(),
            message: format!("scope must be \"global\" or \"project\", got \"{}\"", other),
        }),
    }
}

pub(crate) fn resolve_preset_icon_url(
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
    icon: Option<&str>,
) -> Option<String> {
    let filename = icon?;

    let user_path = app_data_dir.join("agent-icons").join(filename);
    if user_path.is_file() {
        return Some(user_path.to_string_lossy().into_owned());
    }

    if let Some(resource_dir) = resource_dir {
        let bundled = resource_dir.join("agent-icons").join(filename);
        if bundled.is_file() {
            return Some(bundled.to_string_lossy().into_owned());
        }
    }

    None
}

pub(crate) fn preset_to_dto(
    preset: &AgentPreset,
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
) -> AgentPresetDto {
    AgentPresetDto {
        id: preset.id.clone(),
        display_name: preset.display_name.clone(),
        global_path: preset.global_path.clone(),
        project_relative_path: preset.project_relative_path.clone(),
        icon_url: resolve_preset_icon_url(
            app_data_dir,
            resource_dir,
            preset.icon.as_deref(),
        ),
    }
}

pub(crate) fn list_agent_presets_for_scope(
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
    scope: TargetScope,
) -> Result<Vec<AgentPresetDto>, AppError> {
    let presets = agent_presets::load_merged_presets(app_data_dir)?;
    Ok(agent_presets::presets_for_scope(&presets, scope)
        .into_iter()
        .map(|preset| preset_to_dto(preset, app_data_dir, resource_dir))
        .collect())
}

#[tauri::command]
pub fn get_app_state(
    app: tauri::AppHandle,
    selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    let last_migration_report = crate::config_store::take_last_migration_report()
        .map(MigrationReportDto::from);
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    let mut state = build_app_state(config, selected_target_id, Some(app_data_dir.as_path()))
        .map_err(|err| err.to_dto())?;
    state.last_migration_report = last_migration_report;
    Ok(state)
}

#[tauri::command]
pub fn set_main_skills_dir(
    app: tauri::AppHandle,
    path: String,
) -> Result<AppState, AppErrorDto> {
    let path = normalize_platform_path(PathBuf::from(path));
    if !path.exists() {
        return Err(AppError::InvalidMainSkillsDir {
            path: path.clone(),
            message: "路径不存在：{}".to_string(),
        }.to_dto());
    }
    if !path.is_dir() {
        return Err(AppError::InvalidMainSkillsDir {
            path: path.clone(),
            message: "路径不是目录：{}".to_string(),
        }.to_dto());
    }

    run_with_config(
        app,
        |config| {
            config.settings.main_skills_dir = Some(path.clone());
            Ok(())
        },
        None,
        true,
    )
}

#[tauri::command]
pub fn list_agent_presets(
    app: tauri::AppHandle,
    scope: String,
    _project_id: Option<String>,
) -> Result<Vec<AgentPresetDto>, AppErrorDto> {
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    let resource_dir = resource_dir_from_app(&app);
    let scope = parse_target_scope(&scope).map_err(|err| err.to_dto())?;
    list_agent_presets_for_scope(
        &app_data_dir,
        resource_dir.as_deref(),
        scope,
    )
    .map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn add_agent_target(
    app: tauri::AppHandle,
    scope: String,
    agent_id: String,
    project_id: Option<String>,
    _selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto> {
    let scope = parse_target_scope(&scope).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    let presets = agent_presets::load_merged_presets(&app_data_dir).map_err(|err| err.to_dto())?;
    let request = crate::target_registry::AddAgentTargetRequest {
        scope,
        agent_id,
        project_id,
    };

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let target =
        crate::target_registry::add_agent_target(&mut config, &presets, request)
            .map_err(|err| err.to_dto())?;
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    build_app_state_with_mode(
        config,
        Some(target.id),
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn add_custom_target(
    app: tauri::AppHandle,
    scope: String,
    name: String,
    skills_dir: String,
    project_id: Option<String>,
    _selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto> {
    let scope = parse_target_scope(&scope).map_err(|err| err.to_dto())?;
    let request = crate::target_registry::AddCustomTargetRequest {
        scope,
        name,
        skills_dir: PathBuf::from(skills_dir),
        project_id,
    };

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let target = crate::target_registry::add_custom_target(&mut config, request)
        .map_err(|err| err.to_dto())?;
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    build_app_state_with_mode(
        config,
        Some(target.id),
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn add_project(
    app: tauri::AppHandle,
    name: String,
    root_path: String,
    selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto> {
    let root_path = PathBuf::from(root_path);

    run_with_config(
        app,
        |config| {
            crate::project_registry::add_project(config, name, root_path)?;
            Ok(())
        },
        selected_target_id,
        false,
    )
}

#[tauri::command]
pub fn update_project(
    app: tauri::AppHandle,
    project_id: String,
    name: String,
    selected_target_id: Option<String>,
) -> Result<AppState, AppErrorDto> {
    let project_id_for_closure = project_id.clone();

    run_with_config(
        app,
        move |config| {
            crate::project_registry::update_project(config, &project_id_for_closure, name)?;
            Ok(())
        },
        selected_target_id,
        false,
    )
}

#[tauri::command]
pub fn delete_project(
    app: tauri::AppHandle,
    project_id: String,
    selected_target_id: Option<String>,
    cleanup_recorded_links: Option<bool>,
) -> Result<AppState, AppErrorDto> {
    let force = cleanup_recorded_links.unwrap_or(false);
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let warnings = crate::project_registry::delete_project(&mut config, &project_id, force)
        .map_err(|err| err.to_dto())?;

    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;

    let mut state = build_app_state_with_mode(
        config,
        selected_target_id,
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())?;
    state.cleanup_warnings = warnings;
    Ok(state)
}

#[tauri::command]
pub fn update_target(
    app: tauri::AppHandle,
    target_id: String,
    name: String,
) -> Result<AppState, AppErrorDto> {
    let request = crate::target_registry::UpdateTargetRequest { name };
    let target_id_for_closure = target_id.clone();

    run_with_config(
        app,
        move |config| {
            crate::target_registry::update_target(config, &target_id_for_closure, request.clone())?;
            Ok(())
        },
        Some(target_id),
        false,
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
        }
        .to_dto());
    }

    let mut warnings = Vec::new();
    if cleanup_recorded_links {
        warnings = crate::link_installer::force_cleanup_target_installations(&mut config, &target_id);
    }

    crate::target_registry::delete_target_config(&mut config, &target_id)
        .map_err(|err| err.to_dto())?;
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;

    let mut state = build_app_state_with_mode(
        config,
        None,
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())?;
    state.cleanup_warnings = warnings;
    Ok(state)
}

#[tauri::command]
pub fn install_skill(
    app: tauri::AppHandle,
    target_id: String,
    skill_identifier: String,
) -> Result<AppState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    crate::link_installer::install_skill(&mut config, &target_id, &skill_identifier, &skills)
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    build_app_state_with_mode(
        config,
        Some(target_id),
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn sync_target_installations(
    app: tauri::AppHandle,
    source_target_id: String,
    dest_target_id: String,
) -> Result<SyncTargetInstallationsResponse, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    let counts = crate::target_sync::sync_target_installations(
        &mut config,
        &source_target_id,
        &dest_target_id,
        &skills,
    )
    .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;
    let state = build_app_state_with_mode(
        config,
        Some(dest_target_id),
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())?;
    Ok(SyncTargetInstallationsResponse {
        installed: counts.installed,
        skipped: counts.skipped,
        failed: counts.failed,
        state,
    })
}

#[tauri::command]
pub fn uninstall_skill(
    app: tauri::AppHandle,
    target_id: String,
    skill_identifier: String,
    force: Option<bool>,
) -> Result<AppState, AppErrorDto> {
    let force = force.unwrap_or(false);
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let warnings = if force {
        crate::link_installer::force_uninstall_skill(&mut config, &target_id, &skill_identifier)
            .map_err(|err| err.to_dto())?
    } else {
        crate::link_installer::uninstall_skill(&mut config, &target_id, &skill_identifier)
            .map_err(|err| err.to_dto())?;
        Vec::new()
    };

    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    let app_data_dir = app_data_dir_from_app(&app).map_err(|err| err.to_dto())?;

    let mut state = build_app_state_with_mode(
        config,
        Some(target_id),
        Some(app_data_dir.as_path()),
        AppStateBuildMode::Light { skills },
    )
    .map_err(|err| err.to_dto())?;
    state.cleanup_warnings = warnings;
    Ok(state)
}

#[tauri::command]
pub fn delete_main_skill(
    app: tauri::AppHandle,
    skill_identifier: String,
    confirmed: bool,
) -> Result<AppState, AppErrorDto> {
    run_with_config(
        app,
        |config| {
            crate::skill_remover::delete_main_skill(config, &skill_identifier, confirmed)?;
            Ok(())
        },
        None,
        true,
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
                startup_refresh: Default::default(),
            },
            targets,
            installations: Vec::new(),
            ..Default::default()
        }
    }

    fn create_target(id: &str, skills_dir: PathBuf) -> Target {
        Target::global_custom(id, format!("Target {}", id), skills_dir, "1", "1")
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
            link_name: dir_name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_app_state_with_no_targets_returns_empty_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let config = create_config_with_targets(Some(main_dir.clone()), Vec::new());
        let state = build_app_state(config, None, None).expect("build state");

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
        let state = build_app_state(config, None, None).expect("build state");

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
        let state = build_app_state(config, Some("target-2".to_string()), None).expect("build state");

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
        let state = build_app_state(config, Some("nonexistent".to_string()), None).expect("build state");

        assert_eq!(state.selected_target_id, Some("target-1".to_string()));
    }

    #[test]
    fn build_app_state_returns_error_for_invalid_main_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing_dir = temp.path().join("missing");

        let config = create_config_with_targets(Some(missing_dir.clone()), Vec::new());
        let error = build_app_state(config, None, None).expect_err("should fail");

        assert!(matches!(error, AppError::InvalidMainSkillsDir { .. }));
    }

    #[test]
    fn build_app_state_with_no_main_dir_returns_empty_skills() {
        let config = create_config_with_targets(None, Vec::new());
        let state = build_app_state(config, None, None).expect("build state");

        assert!(state.skills.is_empty());
        assert!(state.selected_target_skills.is_empty());
    }

    #[test]
    fn light_build_reuses_skills_without_listing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let _skill = create_valid_skill(&main_dir, "brainstorming");

        let config = create_config_with_targets(Some(main_dir.clone()), Vec::new());
        let full = build_app_state(config.clone(), None, None).expect("full build");
        assert_eq!(full.skills.len(), 1);

        fs::remove_dir_all(&main_dir).expect("remove main dir");
        let light = build_app_state_with_mode(
            config,
            None,
            None,
            AppStateBuildMode::Light {
                skills: full.skills.clone(),
            },
        )
        .expect("light build should not list skills");

        assert_eq!(light.skills, full.skills);
        assert!(light.skills_included);
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
        assert!(dto.message.contains("不能为空"));

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
        assert!(!dto.message.contains("错误 "));
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

    #[test]
    fn parse_target_scope_accepts_global_and_project() {
        assert_eq!(
            parse_target_scope("global").expect("global"),
            crate::models::TargetScope::Global
        );
        assert_eq!(
            parse_target_scope("  PROJECT  ").expect("project"),
            crate::models::TargetScope::Project
        );
    }

    #[test]
    fn parse_target_scope_rejects_unknown_values() {
        let error = parse_target_scope("workspace").expect_err("invalid scope");
        assert!(matches!(error, AppError::InvalidInput { .. }));
    }

    #[test]
    fn list_agent_presets_for_scope_filters_project_presets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let presets = list_agent_presets_for_scope(
            temp.path(),
            None,
            crate::models::TargetScope::Project,
        )
        .expect("list project presets");

        assert_eq!(presets.len(), 3);
        assert!(presets.iter().all(|preset| preset.project_relative_path.is_some()));
    }

    #[test]
    fn resolve_preset_icon_url_prefers_user_override() {
        let temp = tempfile::tempdir().expect("tempdir");
        let icons_dir = temp.path().join("agent-icons");
        fs::create_dir_all(&icons_dir).expect("create icons dir");
        fs::write(icons_dir.join("cursor.png"), b"user icon").expect("write icon");

        let icon_url = resolve_preset_icon_url(temp.path(), None, Some("cursor.png"))
            .expect("icon url");
        assert_eq!(icon_url, icons_dir.join("cursor.png").to_string_lossy().to_string());
    }

    #[test]
    fn add_project_via_registry_updates_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = create_config_with_targets(None, Vec::new());

        crate::project_registry::add_project(
            &mut config,
            "My App".to_string(),
            temp.path().to_path_buf(),
        )
        .expect("add project");

        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].name, "My App");
    }

    #[test]
    fn add_custom_target_global_via_registry() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut config = create_config_with_targets(None, Vec::new());

        let target = crate::target_registry::add_custom_target(
            &mut config,
            crate::target_registry::AddCustomTargetRequest {
                scope: crate::models::TargetScope::Global,
                name: "Tools".to_string(),
                skills_dir: temp.path().to_path_buf(),
                project_id: None,
            },
        )
        .expect("add custom target");

        assert_eq!(target.scope, crate::models::TargetScope::Global);
        assert_eq!(target.kind, crate::models::TargetKind::Custom);
        assert_eq!(config.targets.len(), 1);
    }

    #[test]
    fn update_target_renames_custom_target_without_changing_path() {
        let first = tempfile::tempdir().expect("first tempdir");
        let mut config = create_config_with_targets(None, vec![create_target("target-1", first.path().to_path_buf())]);

        let target = crate::target_registry::update_target(
            &mut config,
            "target-1",
            crate::target_registry::UpdateTargetRequest {
                name: "Renamed".to_string(),
            },
        )
        .expect("update target");

        assert_eq!(target.name, "Renamed");
        assert_eq!(target.skills_dir, first.path());
    }
}
