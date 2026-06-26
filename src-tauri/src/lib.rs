pub mod commands;
pub mod config_store;
pub mod fs_adapter;
pub mod link_installer;
pub mod models;
pub mod skill_library;
pub mod skill_remover;
pub mod target_registry;

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod integration_tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_main_skills_dir,
            commands::add_target,
            commands::update_target,
            commands::delete_target,
            commands::install_skill,
            commands::uninstall_skill,
            commands::delete_main_skill,
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
