mod commands;
mod config;
mod store;

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let store = store::Store::initialize(app.handle())?;
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_config,
            commands::get_store_status,
            commands::ping
        ])
        .run(tauri::generate_context!())
}
