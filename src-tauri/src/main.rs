mod commands;
mod config;

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_app_config,
            commands::ping
        ])
        .run(tauri::generate_context!())
}
