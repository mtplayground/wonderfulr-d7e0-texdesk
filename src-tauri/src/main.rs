mod commands;
mod compile;
mod config;
mod fs;
mod store;
mod watcher;

use tauri::Manager;

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let store = store::Store::initialize(app.handle())?;
            app.manage(store);
            app.manage(watcher::WorkspaceWatcherState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::apply_template_to_workspace,
            commands::compile_document,
            commands::create_workspace_directory,
            commands::create_workspace_file,
            commands::delete_template,
            commands::delete_workspace_entry,
            commands::get_app_config,
            commands::get_store_status,
            commands::get_workspace_watcher_status,
            commands::get_workspace_state,
            commands::list_workspace_entries,
            commands::list_recent_projects,
            commands::list_snippets,
            commands::list_templates,
            commands::ping,
            commands::read_workspace_file,
            commands::remember_open_file,
            commands::remember_workspace_root,
            commands::rename_workspace_entry,
            commands::save_template,
            commands::start_workspace_watcher,
            commands::stop_workspace_watcher,
            commands::write_workspace_file
        ])
        .run(tauri::generate_context!())
}
