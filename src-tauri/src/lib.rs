pub mod commands;
pub mod config_store;
pub mod fs_adapter;
pub mod link_installer;
pub mod models;
pub mod skill_discover;
pub mod skill_downloader;
pub mod skill_install;
pub mod skill_library;
pub mod skill_remover;
pub mod skill_repos;
pub mod skill_smart_paste;
pub mod skill_updates;
pub mod target_registry;

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod integration_tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_main_skills_dir,
            commands::add_target,
            commands::update_target,
            commands::delete_target,
            commands::install_skill,
            commands::uninstall_skill,
            commands::delete_main_skill,
            commands::skill_hub::scan_main_library,
            commands::skill_hub::get_target_skill_states,
            commands::skill_hub::discover_skills,
            commands::skill_hub::install_hub_skill,
            commands::skill_hub::check_skill_updates,
            commands::skill_hub::update_skill,
            commands::skill_hub::update_all_skills,
            commands::skill_hub::parse_smart_paste,
            commands::skill_hub::search_skills_sh,
            commands::skill_hub::get_skill_repos,
            commands::skill_hub::add_skill_repo,
            commands::skill_hub::remove_skill_repo,
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
