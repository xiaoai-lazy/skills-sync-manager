pub mod agent_presets;
pub mod commands;
pub mod config_store;
pub mod credential_store;
pub mod gitlab_client;
pub mod iflytek_skill_hub_client;
pub mod iflytek_skill_hub_discover;
pub mod iflytek_skill_hub_endpoints;
pub mod skill_hub_client;
pub mod skill_hub_discover;
pub mod skill_hub_endpoints;
pub mod skill_hub_upload;
pub mod fs_adapter;
pub mod link_installer;
pub mod models;
pub mod project_registry;
pub mod remote_head;
pub mod repo_cache;
pub mod runtime_cache;
pub mod skill_discover;
pub mod skill_downloader;
pub mod skill_install;
pub mod skill_library;
pub mod skill_markdown;
pub mod skill_migration;
pub mod skill_remover;
pub mod skill_repos;
pub mod skill_smart_paste;
pub mod skill_storage;
pub mod storage_keys;
pub mod skill_updates;
pub mod startup_refresh;
pub mod target_registry;
pub mod target_sync;
pub mod time_util;

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod integration_tests;

use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::updater::PendingUpdate(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_main_skills_dir,
            commands::list_agent_presets,
            commands::add_agent_target,
            commands::add_custom_target,
            commands::add_project,
            commands::update_project,
            commands::delete_project,
            commands::update_target,
            commands::delete_target,
            commands::install_skill,
            commands::sync_target_installations,
            commands::uninstall_skill,
            commands::delete_main_skill,
            commands::skill_hub::scan_main_library,
            commands::skill_hub::get_target_skill_states,
            commands::skill_hub::read_skill_markdown,
            commands::skill_hub::discover_skills,
            commands::skill_hub::refresh_startup_skill_sources,
            commands::skill_hub::set_startup_refresh_settings,
            commands::skill_hub::install_hub_skill,
            commands::skill_hub::check_skill_updates,
            commands::skill_hub::update_skill,
            commands::skill_hub::update_all_skills,
            commands::skill_hub::parse_smart_paste,
            commands::skill_hub::get_skill_repos,
            commands::skill_hub::preview_add_skill_repo,
            commands::skill_hub::validate_gitlab_pat,
            commands::skill_hub::list_gitlab_credentials,
            commands::skill_hub::remove_gitlab_credential,
            commands::skill_hub::update_gitlab_credential,
            commands::skill_hub::add_skill_repo,
            commands::skill_hub::remove_skill_repo,
            commands::skill_hub::set_skill_repo_enabled,
            commands::skill_hub::list_skill_hub_endpoints,
            commands::skill_hub::add_skill_hub_endpoint,
            commands::skill_hub::remove_skill_hub_endpoint,
            commands::skill_hub::set_skill_hub_endpoint_enabled,
            commands::skill_hub::list_iflytek_skill_hub_endpoints,
            commands::skill_hub::add_iflytek_skill_hub_endpoint,
            commands::skill_hub::remove_iflytek_skill_hub_endpoint,
            commands::skill_hub::set_iflytek_skill_hub_endpoint_enabled,
            commands::skill_hub::list_hub_groups,
            commands::skill_hub::create_hub_group,
            commands::skill_hub::upload_skill_to_hub,
            commands::updater::check_app_update,
            commands::updater::install_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skills Sync Manager");
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_backend_test_runs() {
        assert_eq!(2 + 2, 4);
    }
}
