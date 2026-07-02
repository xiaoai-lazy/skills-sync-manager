use crate::commands::store_from_app;
use crate::credential_store;
use crate::gitlab_client;
use crate::models::{
    AppConfig, AppError, AppErrorDto, DiscoverableSkill, DiscoverSkillsResult,
    PreviewAddRepoResult, SkillDiscoverCache,
    SkillHubLocalState, SkillRepo, SkillRepoChangeResult, SkillUpdateInfo, SkillWithTargetState,
    SmartPastePreview, UpdateAllSkillsResult,
};
use crate::skill_repos;
use crate::skill_discover::{
    discover_available_with_warnings, iso8601_timestamp_now, merge_repo_into_discover_cache,
    remove_repo_from_discover_cache,
};
use crate::skill_install;
use crate::skill_library;
use crate::skill_smart_paste;
use crate::skill_updates::{self, apply_check_updates_cache};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

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

fn execute_discover(config: &mut AppConfig) -> (Vec<DiscoverableSkill>, Vec<String>) {
    let main_dir = config.settings.main_skills_dir.as_deref();
    let (skills, warnings) = if config.skill_repos.is_empty()
        || !config.skill_repos.iter().any(|repo| repo.enabled)
    {
        (Vec::new(), Vec::new())
    } else {
        discover_available_with_warnings(config, main_dir)
    };

    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some(iso8601_timestamp_now()),
        skills: skills.clone(),
    };

    (skills, warnings)
}

async fn run_discover_task(
    config: AppConfig,
) -> Result<(AppConfig, DiscoverSkillsResult), AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut config = config;
        let (skills, warnings) = execute_discover(&mut config);
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
pub async fn discover_skills(app: AppHandle) -> Result<DiscoverSkillsResult, AppErrorDto> {
    let _guard = try_begin_discover().map_err(|err| err.to_dto())?;

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let config = store.load().map_err(|err| err.to_dto())?;
    let (config, result) = run_discover_task(config).await.map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(result)
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

pub fn build_skill_hub_local_state(
    main_dir: &Path,
    config: &AppConfig,
) -> Result<SkillHubLocalState, AppError> {
    let skills = skill_library::list_skills(Some(main_dir))?;
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
    let config = store.load().map_err(|err| err.to_dto())?;
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
pub async fn install_hub_skill(
    app: AppHandle,
    discoverable: DiscoverableSkill,
) -> Result<SkillHubLocalState, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    let install_result = tauri::async_runtime::spawn_blocking(move || {
        skill_install::install_to_main(&mut config, &discoverable, &main_dir)?;
        Ok::<_, AppError>((config, main_dir))
    })
    .await
    .map_err(|err| AppErrorDto {
        code: "install_task_failed".to_string(),
        message: format!("安装任务异常: {}", err),
    })?;

    let (config, main_dir) = install_result.map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;
    build_skill_hub_local_state(&main_dir, &config).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn check_skill_updates(app: AppHandle) -> Result<Vec<SkillUpdateInfo>, AppErrorDto> {
    let _guard = try_begin_updates_check().map_err(|err| err.to_dto())?;

    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    if config.skill_records.is_empty() {
        apply_check_updates_cache(&mut config, Vec::new());
        store.save(&config).map_err(|err| err.to_dto())?;
        return Ok(Vec::new());
    }

    let updates = skill_updates::check_updates(&config, &main_dir).map_err(|err| err.to_dto())?;
    apply_check_updates_cache(&mut config, updates.clone());
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(updates)
}

#[tauri::command]
pub fn update_skill(app: AppHandle, dir_name: String) -> Result<(), AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    skill_updates::update_skill(&mut config, &dir_name, &main_dir).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(())
}

#[tauri::command]
pub fn update_all_skills(app: AppHandle) -> Result<UpdateAllSkillsResult, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    let main_dir = require_main_skills_dir(&config)
        .map_err(|err| err.to_dto())?
        .to_path_buf();

    let result =
        skill_updates::update_all_skills(&mut config, &main_dir).map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(result)
}

#[tauri::command]
pub fn parse_smart_paste(input: String) -> Result<SmartPastePreview, AppErrorDto> {
    skill_smart_paste::parse_smart_paste(&input).map_err(|err| err.to_dto())
}

#[tauri::command]
pub fn search_skills_sh(
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<DiscoverableSkill>, AppErrorDto> {
    skill_smart_paste::search_skills_sh(
        &query,
        limit.unwrap_or(20),
        offset.unwrap_or(0),
    )
    .map_err(|err| err.to_dto())
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
    let config = store.load().map_err(|err| err.to_dto())?;
    Ok(credential_store::list_configured_gitlab_hosts(&config))
}

#[tauri::command]
pub fn remove_gitlab_credential(app: AppHandle, host: String) -> Result<(), AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    credential_store::unregister_gitlab_host(&mut config, &host);
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

    let main_dir = config.settings.main_skills_dir.clone();
    let (config, discover_skills) = tauri::async_runtime::spawn_blocking(move || {
        let main_dir = main_dir.as_deref().map(Path::new);
        let discover_skills =
            merge_repo_into_discover_cache(&mut config, &added_repo, main_dir)?;
        Ok::<(AppConfig, Vec<DiscoverableSkill>), AppError>((config, discover_skills))
    })
    .await
    .map_err(|err| AppError::Io {
        path: None,
        message: format!("扫描新来源仓库任务异常: {}", err),
    })
    .map_err(|err| err.to_dto())?
    .map_err(|err| err.to_dto())?;

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
    let mut config = store.load().map_err(|err| err.to_dto())?;

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

    let main_dir = config.settings.main_skills_dir.clone();
    let host_for_discover = host;
    let project_path_for_discover = project_path;

    let (config, discover_skills) = tauri::async_runtime::spawn_blocking(move || {
        let main_dir = main_dir.as_deref().map(Path::new);
        let discover_skills = if enabled {
            merge_repo_into_discover_cache(&mut config, &repo, main_dir)?
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

    store.save(&config).map_err(|err| err.to_dto())?;

    Ok(SkillRepoChangeResult {
        repos: skill_repos::get_skill_repos(&config),
        discover_skills,
    })
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
                },
                SkillUpdateInfo {
                    dir_name: "valid-two".to_string(),
                    name: "valid-two".to_string(),
                    current_hash: Some("111".to_string()),
                    remote_hash: "222".to_string(),
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
}
