pub mod config_store;
pub mod fs_adapter;
pub mod link_installer;
pub mod models;
pub mod skill_library;
pub mod skill_remover;
pub mod target_registry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
