mod commands;
mod config;
mod fs;
mod store;

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let store = store::Store::initialize(app.handle())?;
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_workspace_directory,
            commands::create_workspace_file,
            commands::delete_workspace_entry,
            commands::get_app_config,
            commands::get_store_status,
            commands::list_workspace_entries,
            commands::ping,
            commands::read_workspace_file,
            commands::rename_workspace_entry,
            commands::write_workspace_file
        ])
        .run(tauri::generate_context!())
}
