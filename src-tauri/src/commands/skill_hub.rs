use crate::commands::store_from_app;
use crate::credential_store;
use crate::gitlab_client;
use crate::models::{
    AppConfig, AppError, AppErrorDto, DiscoverableSkill, DiscoverSkillsResult,
    PreviewAddRepoResult, SkillDiscoverCache,
    SkillHubEndpoint, SkillHubEndpointChangeResult, SkillHubLocalState,
    SkillMarkdownPreviewDto, SkillMarkdownRequestDto, SkillRepo, SkillRepoChangeResult,
    SkillUpdateInfo, SkillWithTargetState, StartupRefreshSettings, StartupSkillRefreshResult,
    SmartPastePreview, UpdateAllSkillsResult,
};
use crate::skill_hub_endpoints;
use crate::skill_repos;
use crate::skill_discover::{
    deduplicate_discoverable_skills, discover_available_with_warnings,
    filter_uninstalled_discoverable_skills, iso8601_timestamp_now,
    merge_repo_into_discover_cache, remove_repo_from_discover_cache,
};
use crate::skill_hub_client;
use crate::skill_hub_discover;
use crate::skill_hub_upload;
use crate::skill_install;
use crate::skill_library;
use crate::skill_smart_paste;
use crate::skill_updates::{self, apply_check_updates_cache};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

static DISCOVER_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static UPDATES_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

struct DiscoverGuard;

impl Drop for DiscoverGuard {
    fn drop(&mut self) {
        DISCOVER_IN_PROGRESS.store(false, Ordering::Release);
    }
}

fn try_begin_discover() -> Result<DiscoverGuard, AppError> {
    if DISCOVER_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(AppError::DiscoverInProgress);
    }

    Ok(DiscoverGuard)
}

struct UpdatesGuard;

impl Drop for UpdatesGuard {
    fn drop(&mut self) {
        UPDATES_IN_PROGRESS.store(false, Ordering::Release);
    }
}

fn try_begin_updates_check() -> Result<UpdatesGuard, AppError> {
    if UPDATES_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(AppError::UpdatesInProgress);
    }

    Ok(UpdatesGuard)
}

fn execute_discover(
    config: &mut AppConfig,
    app_data_dir: &Path,
    force: bool,
) -> (Vec<DiscoverableSkill>, Vec<String>) {
    let main_dir = config.settings.main_skills_dir.as_deref();
    let mut all = Vec::new();
    let mut warnings = Vec::new();

    if config.skill_repos.iter().any(|repo| repo.enabled) {
        let (repo_skills, repo_warnings) =
            discover_available_with_warnings(config, main_dir, app_data_dir, force);
        all.extend(repo_skills);
        warnings.extend(repo_warnings);
    }

    if config
        .skill_hub_endpoints
        .iter()
        .any(|endpoint| endpoint.enabled)
    {
        let (hub_skills, hub_warnings) = skill_hub_discover::discover_all(config);
        all.extend(hub_skills);
        warnings.extend(hub_warnings);
    }

    let skills = deduplicate_discoverable_skills(filter_uninstalled_discoverable_skills(
        all,
        main_dir,
        Some(&config.skill_records),
    ));

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    (skills, warnings)
}

async fn run_discover_task(
    config: AppConfig,
    app_data_dir: PathBuf,
    force: bool,
) -> Result<(AppConfig, DiscoverSkillsResult), AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut config = config;
        let (skills, warnings) = execute_discover(&mut config, &app_data_dir, force);
        Ok((
            config,
            DiscoverSkillsResult { skills, warnings },
        ))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("刷新列表任务异常: {}", err),
    })?
}

#[tauri::command]
pub async fn discover_skills(
    app: AppHandle,
    force: Option<bool>,
) -> Result<DiscoverSkillsResult, AppErrorDto> {
    let _guard = try_begin_discover().map_err(|err| err.to_dto())?;
    let force = force.unwrap_or(false);

    let app_data_dir = app.path().app_data_dir().map_err(|err| AppError::Io {
        path: None,
        message: format!("failed to resolve app data directory: {}", err),
    }).map_err(|err| err.to_dto())?;

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let (config, result) = run_discover_task(config, app_data_dir.clone(), force)
        .await
        .map_err(|err| err.to_dto())?;
    crate::runtime_cache::persist_from_config(&app_data_dir, &config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(result)
}

#[tauri::command]
pub async fn refresh_startup_skill_sources(
    app: AppHandle,
) -> Result<StartupSkillRefreshResult, AppErrorDto> {
    let _discover_guard = try_begin_discover().map_err(|err| err.to_dto())?;
    let _updates_guard = try_begin_updates_check().map_err(|err| err.to_dto())?;
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = config.settings.main_skills_dir.clone();
    let task_app_data_dir = app_data_dir.clone();

    let (refreshed, result) = tauri::async_runtime::spawn_blocking(move || {
        let result = crate::startup_refresh::refresh_enabled_sources(
            &mut config,
            main_dir.as_deref(),
            &task_app_data_dir,
        );
        (config, result)
    })
    .await
    .map_err(|err| AppErrorDto {
        code: "startupRefreshTaskFailed".to_string(),
        message: format!("启动刷新任务异常: {}", err),
    })?;

    let mut latest = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut latest);
    latest.skill_discover_cache = refreshed.skill_discover_cache;
    latest.skill_update_cache = refreshed.skill_update_cache;
    crate::runtime_cache::persist_from_config(&app_data_dir, &latest)
        .map_err(|err| err.to_dto())?;
    store.save(&latest).map_err(|err| err.to_dto())?;

    Ok(result)
}

#[tauri::command]
pub fn set_startup_refresh_settings(
    app: AppHandle,
    settings: StartupRefreshSettings,
) -> Result<StartupRefreshSettings, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    apply_startup_refresh_settings(&mut config, settings.clone());
    store.save(&config).map_err(|err| err.to_dto())?;
    Ok(settings)
}

fn apply_startup_refresh_settings(config: &mut AppConfig, settings: StartupRefreshSettings) {
    config.settings.startup_refresh = settings;
}

fn require_main_skills_dir(config: &AppConfig) -> Result<&Path, AppError> {
    let main_dir = config
        .settings
        .main_skills_dir
        .as_deref()
        .ok_or_else(|| AppError::InvalidMainSkillsDir {
            path: PathBuf::new(),
            message: "主 skill 目录未配置".to_string(),
        })?;

    if !main_dir.is_dir() {
        return Err(AppError::InvalidMainSkillsDir {
            path: main_dir.to_path_buf(),
            message: "主 skill 目录不存在或无效".to_string(),
        });
    }

    Ok(main_dir)
}

fn annotate_local_dirty(skills: &mut [crate::models::SkillView], config: &AppConfig) {
    for skill in skills.iter_mut() {
        skill.local_dirty = false;
        if skill.storage_key.is_empty() || !skill.path.is_dir() {
            continue;
        }
        let Some(record) = config.skill_records.get(&skill.storage_key).or_else(|| {
            config
                .skill_records
                .values()
                .find(|r| r.storage_key == skill.storage_key)
        }) else {
            continue;
        };
        if record.source != "skillhub" || record.content_hash.is_empty() {
            continue;
        }
        let Ok(current) = skill_updates::hash_matching_stored_content_hash(&skill.path, &record.content_hash)
        else {
            continue;
        };
        skill.local_dirty = current != record.content_hash;
    }
}

pub fn build_skill_hub_local_state(
    main_dir: &Path,
    config: &AppConfig,
) -> Result<SkillHubLocalState, AppError> {
    let mut skills = skill_library::list_skills(Some(main_dir))?;
    annotate_local_dirty(&mut skills, config);
    let valid_count = skills.iter().filter(|skill| skill.valid).count() as u32;
    let invalid_count = skills.len() as u32 - valid_count;

    Ok(SkillHubLocalState {
        skills,
        valid_count,
        invalid_count,
        pending_update_count: config.skill_update_cache.updates.len() as u32,
        last_scan_at: iso8601_timestamp_now(),
        skill_records: config.skill_records.clone(),
    })
}

#[tauri::command]
pub fn scan_main_library(app: AppHandle) -> Result<SkillHubLocalState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = require_main_skills_dir(&config).map_err(|err| err.to_dto())?;
    build_skill_hub_local_state(main_dir, &config).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn get_target_skill_states(
    app: AppHandle,
    target_id: String,
) -> Result<Vec<SkillWithTargetState>, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    let skills = skill_library::list_skills(config.settings.main_skills_dir.as_deref())
        .map_err(|err| err.to_dto())?;
    crate::link_installer::compute_target_skill_states(&config, &target_id, &skills)
        .map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn read_skill_markdown(
    app: AppHandle,
    request: SkillMarkdownRequestDto,
) -> Result<SkillMarkdownPreviewDto, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    crate::skill_markdown::read_skill_markdown(&config, &app_data_dir, request)
        .map_err(|err| err.to_dto())
}

#[tauri::command]
pub async fn install_hub_skill(
    app: AppHandle,
    discoverable: DiscoverableSkill,
) -> Result<SkillHubLocalState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    let install_result = tauri::async_runtime::spawn_blocking(move || {
        let result = skill_install::install_to_main(&mut config, &discoverable, &main_dir);
        Ok::<_, AppError>((config, main_dir, result))
    })
    .await
    .map_err(|err| AppErrorDto {
        code: "install_task_failed".to_string(),
        message: format!("安装任务异常: {}", err),
    })?;

    let (config, main_dir, result) = install_result.map_err(|err| err.to_dto())?;
    match result {
        Ok(()) => {
            crate::runtime_cache::persist_from_config(&app_data_dir, &config)
                .map_err(|err| err.to_dto())?;
            store.save(&config).map_err(|err| err.to_dto())?;
            build_skill_hub_local_state(&main_dir, &config).map_err(|err| err.to_dto())
        }
        Err(err) => {
            // HubSkillGone already purged discover cache in-memory; persist that cleanup.
            if matches!(err, AppError::HubSkillGone { .. }) {
                let _ = crate::runtime_cache::persist_from_config(&app_data_dir, &config);
                let _ = store.save(&config);
            }
            Err(err.to_dto())
        }
    }
}

#[tauri::command]
pub fn check_skill_updates(app: AppHandle) -> Result<Vec<SkillUpdateInfo>, AppErrorDto> {
    let _guard = try_begin_updates_check().map_err(|err| err.to_dto())?;

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    // Updates cache lives in runtime-cache; attach before read/modify.
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    if config.skill_records.is_empty() {
        apply_check_updates_cache(&mut config, Vec::new());
        crate::runtime_cache::persist_from_config(&app_data_dir, &config)
            .map_err(|err| err.to_dto())?;
        store.save(&config).map_err(|err| err.to_dto())?;
        return Ok(Vec::new());
    }

    let updates = skill_updates::check_updates(&mut config, &main_dir).map_err(|err| err.to_dto())?;
    apply_check_updates_cache(&mut config, updates.clone());
    crate::runtime_cache::persist_from_config(&app_data_dir, &config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(updates)
}

#[tauri::command]
pub fn update_skill(app: AppHandle, dir_name: String) -> Result<(), AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    match skill_updates::update_skill(&mut config, &dir_name, &main_dir) {
        Ok(()) => {
            crate::runtime_cache::persist_from_config(&app_data_dir, &config)
                .map_err(|err| err.to_dto())?;
            store.save(&config).map_err(|err| err.to_dto())?;
            Ok(())
        }
        Err(err) => {
            if matches!(err, AppError::HubSkillGone { .. }) {
                let _ = crate::runtime_cache::persist_from_config(&app_data_dir, &config);
                let _ = store.save(&config);
            }
            Err(err.to_dto())
        }
    }
}

#[tauri::command]
pub fn update_all_skills(app: AppHandle) -> Result<UpdateAllSkillsResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    let result =
        skill_updates::update_all_skills(&mut config, &main_dir).map_err(|err| err.to_dto())?;
    crate::runtime_cache::persist_from_config(&app_data_dir, &config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(result)
}

#[tauri::command]
pub fn parse_smart_paste(input: String) -> Result<SmartPastePreview, AppErrorDto> {
    skill_smart_paste::parse_smart_paste(&input).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn get_skill_repos(app: AppHandle) -> Result<Vec<SkillRepo>, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    Ok(skill_repos::get_skill_repos(&config))
}

#[tauri::command]
pub fn preview_add_skill_repo(app: AppHandle, url: String) -> Result<PreviewAddRepoResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    Ok(skill_repos::preview_add_skill_repo(&config, &url))
}

#[tauri::command]
pub fn validate_gitlab_pat(app: AppHandle, host: String, pat: String) -> Result<(), AppErrorDto> {
    let _store = store_from_app(&app).map_err(|err| err.to_dto())?;
    gitlab_client::validate_token(&host, &pat).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn list_gitlab_credentials(app: AppHandle) -> Result<Vec<String>, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    if credential_store::reconcile_gitlab_credential_hosts(&mut config) {
        store.save(&config).map_err(|err| err.to_dto())?;
    }
    Ok(credential_store::list_configured_gitlab_hosts(&config))
}

#[tauri::command]
pub fn remove_gitlab_credential(app: AppHandle, host: String) -> Result<(), AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    credential_store::unregister_gitlab_host(&mut config, &host)
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn update_gitlab_credential(
    app: AppHandle,
    host: String,
    pat: String,
) -> Result<(), AppErrorDto> {
    gitlab_client::validate_token(&host, &pat).map_err(|err| err.to_dto())?;
    credential_store::set_gitlab_token(&host, &pat).map_err(|err| err.to_dto())?;

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    credential_store::register_gitlab_host(&mut config, &host);
    store.save(&config).map_err(|err| err.to_dto())
}

#[tauri::command]
pub async fn add_skill_repo(
    app: AppHandle,
    url: String,
    branch: Option<String>,
    pat: Option<String>,
) -> Result<SkillRepoChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let url_for_task = url;
    let branch_for_task = branch;
    let pat_for_task = pat;

    let (mut config, added_repo) = tauri::async_runtime::spawn_blocking(move || {
        let added_repo = skill_repos::add_skill_repo(
            &mut config,
            &url_for_task,
            branch_for_task.as_deref(),
            pat_for_task.as_deref(),
        )?;
        Ok::<(AppConfig, SkillRepo), AppError>((config, added_repo))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("添加来源仓库任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    store.save(&config).map_err(|err| err.to_dto())?;

    let app_data_dir = app.path().app_data_dir().map_err(|err| AppError::Io {
        path: None,
        message: format!("failed to resolve app data directory: {}", err),
    }).map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = config.settings.main_skills_dir.clone();
    let app_data_dir_for_task = app_data_dir.clone();
    let (config, discover_skills) = tauri::async_runtime::spawn_blocking(move || {
        let main_dir = main_dir.as_deref().map(Path::new);
        let discover_skills = merge_repo_into_discover_cache(
            &mut config,
            &added_repo,
            main_dir,
            &app_data_dir_for_task,
            true,
        )?;
        Ok::<(AppConfig, Vec<DiscoverableSkill>), AppError>((config, discover_skills))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("扫描新来源仓库任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    crate::runtime_cache::persist_from_config(&app_data_dir, &config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(SkillRepoChangeResult {
        repos: skill_repos::get_skill_repos(&config),
        discover_skills,
    })
}

#[tauri::command]
pub async fn remove_skill_repo(
    app: AppHandle,
    host: String,
    project_path: String,
) -> Result<SkillRepoChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);

    let host_for_task = host;
    let project_path_for_task = project_path;

    let (config, discover_skills) = tauri::async_runtime::spawn_blocking(move || {
        skill_repos::remove_skill_repo(&mut config, &host_for_task, &project_path_for_task)?;
        let discover_skills =
            remove_repo_from_discover_cache(&mut config, &host_for_task, &project_path_for_task);
        Ok::<(AppConfig, Vec<DiscoverableSkill>), AppError>((config, discover_skills))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("删除来源仓库任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    crate::runtime_cache::persist_from_config(&app_data_dir, &config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(SkillRepoChangeResult {
        repos: skill_repos::get_skill_repos(&config),
        discover_skills,
    })
}

#[tauri::command]
pub async fn set_skill_repo_enabled(
    app: AppHandle,
    host: String,
    project_path: String,
    enabled: bool,
) -> Result<SkillRepoChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let host_for_task = host.clone();
    let project_path_for_task = project_path.clone();

    let (mut config, repo) = tauri::async_runtime::spawn_blocking(move || {
        let repo = skill_repos::set_skill_repo_enabled(
            &mut config,
            &host_for_task,
            &project_path_for_task,
            enabled,
        )?;
        Ok::<(AppConfig, SkillRepo), AppError>((config, repo))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("更新来源仓库状态任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    store.save(&config).map_err(|err| err.to_dto())?;

    let app_data_dir = app.path().app_data_dir().map_err(|err| AppError::Io {
        path: None,
        message: format!("failed to resolve app data directory: {}", err),
    }).map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    let main_dir = config.settings.main_skills_dir.clone();
    let host_for_discover = host;
    let project_path_for_discover = project_path;
    let app_data_dir_for_task = app_data_dir.clone();

    let (config, discover_skills) = tauri::async_runtime::spawn_blocking(move || {
        let main_dir = main_dir.as_deref().map(Path::new);
        let discover_skills = if enabled {
            merge_repo_into_discover_cache(&mut config, &repo, main_dir, &app_data_dir_for_task, true)?
        } else {
            remove_repo_from_discover_cache(&mut config, &host_for_discover, &project_path_for_discover)
        };
        Ok::<(AppConfig, Vec<DiscoverableSkill>), AppError>((config, discover_skills))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("更新发现列表任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    crate::runtime_cache::persist_from_config(&app_data_dir, &config).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(SkillRepoChangeResult {
        repos: skill_repos::get_skill_repos(&config),
        discover_skills,
    })
}

fn hub_endpoint_change_result(config: &AppConfig) -> SkillHubEndpointChangeResult {
    SkillHubEndpointChangeResult {
        endpoints: skill_hub_endpoints::list_skill_hub_endpoints(config),
        discover_skills: config.skill_discover_cache.skills.clone(),
    }
}

#[tauri::command]
pub fn list_skill_hub_endpoints(app: AppHandle) -> Result<Vec<SkillHubEndpoint>, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    Ok(skill_hub_endpoints::list_skill_hub_endpoints(&config))
}

#[tauri::command]
pub async fn add_skill_hub_endpoint(
    app: AppHandle,
    name: String,
    base_url: String,
) -> Result<SkillHubEndpointChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let name_for_task = name;
    let base_url_for_task = base_url;

    let config = tauri::async_runtime::spawn_blocking(move || {
        skill_hub_endpoints::add_skill_hub_endpoint(
            &mut config,
            &name_for_task,
            &base_url_for_task,
        )?;
        Ok::<AppConfig, AppError>(config)
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("添加 Hub 端点任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    store.save(&config).map_err(|err| err.to_dto())?;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = config;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    Ok(hub_endpoint_change_result(&config))
}

#[tauri::command]
pub fn remove_skill_hub_endpoint(
    app: AppHandle,
    id: String,
) -> Result<SkillHubEndpointChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    skill_hub_endpoints::remove_skill_hub_endpoint(&mut config, &id).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    Ok(hub_endpoint_change_result(&config))
}

#[tauri::command]
pub async fn set_skill_hub_endpoint_enabled(
    app: AppHandle,
    id: String,
    enabled: bool,
) -> Result<SkillHubEndpointChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;

    let id_for_task = id;

    let config = tauri::async_runtime::spawn_blocking(move || {
        skill_hub_endpoints::set_skill_hub_endpoint_enabled(&mut config, &id_for_task, enabled)?;
        Ok::<AppConfig, AppError>(config)
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("更新 Hub 端点状态任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    store.save(&config).map_err(|err| err.to_dto())?;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    let mut config = config;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut config);
    Ok(hub_endpoint_change_result(&config))
}

#[tauri::command]
pub async fn list_hub_groups(
    app: AppHandle,
    hub_endpoint_id: String,
) -> Result<Vec<String>, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    let base_url =
        skill_hub_endpoints::hub_endpoint_base_url(&config, &hub_endpoint_id).map_err(|err| err.to_dto())?;

    tauri::async_runtime::spawn_blocking(move || {
        let groups = skill_hub_client::fetch_groups(&base_url)?;
        Ok::<Vec<String>, AppError>(
            groups
                .into_iter()
                .map(|group| group.name)
                .collect::<Vec<_>>(),
        )
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("获取 Hub 分组任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())
}

#[tauri::command]
pub async fn create_hub_group(
    app: AppHandle,
    hub_endpoint_id: String,
    name: String,
) -> Result<Vec<String>, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    let base_url =
        skill_hub_endpoints::hub_endpoint_base_url(&config, &hub_endpoint_id).map_err(|err| err.to_dto())?;

    tauri::async_runtime::spawn_blocking(move || {
        skill_hub_client::create_group(&base_url, &name)?;
        let groups = skill_hub_client::fetch_groups(&base_url)?;
        Ok::<Vec<String>, AppError>(
            groups
                .into_iter()
                .map(|group| group.name)
                .collect::<Vec<_>>(),
        )
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("创建 Hub 分组任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())
}

#[tauri::command]
pub async fn upload_skill_to_hub(
    app: AppHandle,
    hub_endpoint_id: String,
    group: String,
    storage_key: String,
) -> Result<SkillHubEndpointChangeResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    let hub_endpoint_id_for_task = hub_endpoint_id;
    let group_for_task = group;
    let storage_key_for_merge = storage_key.clone();
    let storage_key_for_task = storage_key;

    let config = tauri::async_runtime::spawn_blocking(move || {
        skill_hub_upload::upload_skill_to_hub(
            &mut config,
            &hub_endpoint_id_for_task,
            &group_for_task,
            &storage_key_for_task,
            &main_dir,
        )?;
        Ok::<AppConfig, AppError>(config)
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("上传 Skill 任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

    // Persist discover cache + only the uploaded skill_record(s); avoid clobbering
    // concurrent skill_records / skill_update_cache writes.
    let mut latest = store.load().map_err(|err| err.to_dto())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io {
            path: None,
            message: format!("failed to resolve app data directory: {}", err),
        })
        .map_err(|err| err.to_dto())?;
    crate::runtime_cache::attach_to_config(&app_data_dir, &mut latest);
    merge_uploaded_skill_records(&mut latest, &config, &storage_key_for_merge);
    latest.skill_discover_cache = config.skill_discover_cache;
    crate::runtime_cache::persist_from_config(&app_data_dir, &latest).map_err(|err| err.to_dto())?;
    store.save(&latest).map_err(|err| err.to_dto())?;

    Ok(hub_endpoint_change_result(&latest))
}

/// Merge only the uploaded skill's record(s) whose content_hash was refreshed.
/// Matches by map key or `SkillRecord.storage_key`; leaves other records and
/// `skill_update_cache` untouched.
pub(crate) fn merge_uploaded_skill_records(
    latest: &mut AppConfig,
    uploaded: &AppConfig,
    storage_key: &str,
) {
    for (key, uploaded_record) in &uploaded.skill_records {
        let matches_uploaded =
            key.as_str() == storage_key || uploaded_record.storage_key == storage_key;
        if !matches_uploaded {
            continue;
        }
        let should_merge = match latest.skill_records.get(key) {
            Some(existing) => existing.content_hash != uploaded_record.content_hash,
            None => true,
        };
        if should_merge {
            latest
                .skill_records
                .insert(key.clone(), uploaded_record.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        SkillInstallState, SkillUpdateCache, SkillUpdateInfo, Target,
    };
    use std::fs;

    fn create_valid_skill(main_dir: &Path, dir_name: &str) {
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
    }

    fn create_invalid_skill(main_dir: &Path, dir_name: &str) {
        let skill_dir = main_dir.join(dir_name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
    }

    fn create_hub_skill(main_dir: &Path, storage_key: &str, name: &str) {
        let skill_dir = crate::skill_storage::main_library_path(main_dir, storage_key);
        fs::create_dir_all(&skill_dir).expect("create hub skill dir");
        fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Test skill.\n---\n\n# Skill\n",
                name
            ),
        )
        .expect("write skill md");
    }

    fn hub_skill_record(storage_key: &str, content_hash: &str) -> crate::models::SkillRecord {
        crate::models::SkillRecord {
            source: "skillhub".to_string(),
            storage_key: storage_key.to_string(),
            content_hash: content_hash.to_string(),
            installed_at: "2026-06-30T00:00:00Z".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_skill_hub_local_state_marks_local_dirty_when_hash_diverges() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let storage_key = "hub/company-hub/common/tdd";
        create_hub_skill(&main_dir, storage_key, "tdd");

        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(main_dir.clone());
        config.skill_records.insert(
            storage_key.to_string(),
            hub_skill_record(storage_key, "stalehash0"),
        );

        let state = build_skill_hub_local_state(&main_dir, &config).expect("build state");
        let skill = state
            .skills
            .iter()
            .find(|s| s.storage_key == storage_key)
            .expect("hub skill");
        assert!(skill.local_dirty);
    }

    #[test]
    fn build_skill_hub_local_state_clears_local_dirty_when_hash_matches() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        let storage_key = "hub/company-hub/common/tdd";
        create_hub_skill(&main_dir, storage_key, "tdd");
        let skill_dir = crate::skill_storage::main_library_path(&main_dir, storage_key);
        let matching_hash =
            skill_updates::compute_skill_md_hash_prefix(&skill_dir).expect("compute hash");

        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(main_dir.clone());
        config.skill_records.insert(
            storage_key.to_string(),
            hub_skill_record(storage_key, &matching_hash),
        );

        let state = build_skill_hub_local_state(&main_dir, &config).expect("build state");
        let skill = state
            .skills
            .iter()
            .find(|s| s.storage_key == storage_key)
            .expect("hub skill");
        assert!(!skill.local_dirty);
    }

    #[test]
    fn build_skill_hub_local_state_non_hub_is_not_local_dirty() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        create_valid_skill(&main_dir, "github-skill");
        let storage_key = "github-skill";

        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(main_dir.clone());
        config.skill_records.insert(
            storage_key.to_string(),
            crate::models::SkillRecord {
                source: "github".to_string(),
                storage_key: storage_key.to_string(),
                content_hash: "stalehash0".to_string(),
                installed_at: "2026-06-30T00:00:00Z".to_string(),
                ..Default::default()
            },
        );

        let state = build_skill_hub_local_state(&main_dir, &config).expect("build state");
        let skill = state
            .skills
            .iter()
            .find(|s| s.storage_key == storage_key)
            .expect("github skill");
        assert!(!skill.local_dirty);
    }

    #[test]
    fn build_skill_hub_local_state_counts_valid_invalid_and_pending_updates() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        create_valid_skill(&main_dir, "valid-one");
        create_valid_skill(&main_dir, "valid-two");
        create_invalid_skill(&main_dir, "invalid-one");

        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(main_dir.clone());
        config.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-01-01T00:00:00Z".to_string()),
            updates: vec![
                SkillUpdateInfo {
                    dir_name: "valid-one".to_string(),
                    name: "valid-one".to_string(),
                    current_hash: Some("abc".to_string()),
                    remote_hash: "def".to_string(),
                    ..Default::default()
                },
                SkillUpdateInfo {
                    dir_name: "valid-two".to_string(),
                    name: "valid-two".to_string(),
                    current_hash: Some("111".to_string()),
                    remote_hash: "222".to_string(),
                    ..Default::default()
                },
            ],
        };
        config.skill_records.insert(
            "valid-one".to_string(),
            crate::models::SkillRecord {
                repo_host: "github.com".to_string(),
                project_path: "anthropics/skills".to_string(),
                source: "github".to_string(),
                repo_owner: "anthropics".to_string(),
                repo_name: "skills".to_string(),
                repo_branch: "main".to_string(),
                directory: "skills/valid-one".to_string(),
                content_hash: "abc".to_string(),
                installed_at: "2026-06-30T00:00:00Z".to_string(),
                ..Default::default()
            },
        );

        let state = build_skill_hub_local_state(&main_dir, &config).expect("build state");

        assert_eq!(state.skills.len(), 3);
        assert_eq!(state.valid_count, 2);
        assert_eq!(state.invalid_count, 1);
        assert_eq!(state.pending_update_count, 2);
        assert!(!state.last_scan_at.is_empty());
        assert_eq!(state.skill_records.get("valid-one").unwrap().source, "github");
    }

    #[test]
    fn get_target_skill_states_returns_installed_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main_dir = temp.path().join("main-skills");
        fs::create_dir_all(&main_dir).expect("create main dir");
        create_valid_skill(&main_dir, "brainstorming");

        let target_dir = temp.path().join("target-skills");
        fs::create_dir_all(&target_dir).expect("create target dir");

        let skills = skill_library::list_skills(Some(&main_dir)).expect("list skills");
        assert_eq!(skills.len(), 1);

        let mut config = AppConfig::default();
        config.settings.main_skills_dir = Some(main_dir);
        config.targets = vec![Target::global_custom(
            "target-1",
            "Target One",
            target_dir.clone(),
            "1",
            "1",
        )];

        crate::link_installer::install_skill(&mut config, "target-1", "brainstorming", &skills)
            .expect("install skill");

        let states = crate::link_installer::compute_target_skill_states(
            &config,
            "target-1",
            &skills,
        )
        .expect("compute states");

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].skill.dir_name, "brainstorming");
        assert_eq!(states[0].state, SkillInstallState::Installed);
    }

    #[test]
    fn merge_uploaded_skill_records_updates_only_matching_hash_changed_keys() {
        let storage_key = "hub/company-hub/common/tdd";
        let other_key = "hub/company-hub/common/other";

        let mut uploaded = AppConfig::default();
        uploaded.skill_records.insert(
            storage_key.to_string(),
            crate::models::SkillRecord {
                source: "skillhub".to_string(),
                storage_key: storage_key.to_string(),
                content_hash: "newhash00001".to_string(),
                ..Default::default()
            },
        );
        uploaded.skill_records.insert(
            other_key.to_string(),
            crate::models::SkillRecord {
                source: "skillhub".to_string(),
                storage_key: other_key.to_string(),
                content_hash: "uploaded-other".to_string(),
                ..Default::default()
            },
        );

        let mut latest = AppConfig::default();
        latest.skill_records.insert(
            storage_key.to_string(),
            crate::models::SkillRecord {
                source: "skillhub".to_string(),
                storage_key: storage_key.to_string(),
                content_hash: "oldhash00001".to_string(),
                ..Default::default()
            },
        );
        latest.skill_records.insert(
            other_key.to_string(),
            crate::models::SkillRecord {
                source: "skillhub".to_string(),
                storage_key: other_key.to_string(),
                content_hash: "latest-other".to_string(),
                ..Default::default()
            },
        );
        latest.skill_records.insert(
            "concurrent-new".to_string(),
            crate::models::SkillRecord {
                source: "github".to_string(),
                content_hash: "keep-me".to_string(),
                ..Default::default()
            },
        );
        latest.skill_update_cache = SkillUpdateCache {
            checked_at: Some("2026-07-20T00:00:00Z".to_string()),
            updates: vec![SkillUpdateInfo {
                dir_name: "tdd".to_string(),
                name: "tdd".to_string(),
                current_hash: Some("a".to_string()),
                remote_hash: "b".to_string(),
                storage_key: storage_key.to_string(),
            }],
        };
        let update_cache_before = latest.skill_update_cache.clone();

        merge_uploaded_skill_records(&mut latest, &uploaded, storage_key);

        assert_eq!(
            latest
                .skill_records
                .get(storage_key)
                .unwrap()
                .content_hash,
            "newhash00001"
        );
        assert_eq!(
            latest.skill_records.get(other_key).unwrap().content_hash,
            "latest-other",
            "unrelated records must not be overwritten from uploaded config"
        );
        assert_eq!(
            latest
                .skill_records
                .get("concurrent-new")
                .unwrap()
                .content_hash,
            "keep-me",
            "concurrent records on latest must be preserved"
        );
        assert_eq!(latest.skill_update_cache, update_cache_before);
    }

    #[test]
    fn merge_uploaded_skill_records_matches_by_record_storage_key() {
        let map_key = "legacy-tdd";
        let storage_key = "hub/company-hub/common/tdd";

        let mut uploaded = AppConfig::default();
        uploaded.skill_records.insert(
            map_key.to_string(),
            crate::models::SkillRecord {
                storage_key: storage_key.to_string(),
                content_hash: "refreshed".to_string(),
                ..Default::default()
            },
        );

        let mut latest = AppConfig::default();
        latest.skill_records.insert(
            map_key.to_string(),
            crate::models::SkillRecord {
                storage_key: storage_key.to_string(),
                content_hash: "stale".to_string(),
                ..Default::default()
            },
        );

        merge_uploaded_skill_records(&mut latest, &uploaded, storage_key);

        assert_eq!(
            latest.skill_records.get(map_key).unwrap().content_hash,
            "refreshed"
        );
    }
}
    #[test]
    fn apply_startup_refresh_settings_changes_only_settings() {
        let mut config = AppConfig::default();
        config.skill_repos.push(SkillRepo {
            host: "github.com".to_string(),
            provider: "github".to_string(),
            project_path: "owner/repo".to_string(),
            owner: "owner".to_string(),
            name: "repo".to_string(),
            branch: "main".to_string(),
            enabled: true,
        });
        config.skill_records.insert(
            "record".to_string(),
            crate::models::SkillRecord::default(),
        );
        let repos = config.skill_repos.clone();
        let records = config.skill_records.clone();
        let settings = StartupRefreshSettings {
            github: true,
            gitlab: false,
            skill_hub: false,
        };

        apply_startup_refresh_settings(&mut config, settings.clone());

        assert_eq!(config.settings.startup_refresh, settings);
        assert_eq!(config.skill_repos, repos);
        assert_eq!(config.skill_records, records);
    }
