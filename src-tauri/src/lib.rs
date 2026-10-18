pub mod audit;
mod commands;
mod desktop;
pub mod error;
pub mod model;
pub mod platform;
pub mod process;
pub mod security;
pub mod service;

pub fn run() {
    if std::env::args().any(|a| a == "--read-only-check") {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let snapshot = runtime.block_on(service::Service::new(platform::native()).snapshot());
        println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            desktop::show_panel(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(desktop::shortcut_plugin())
        .manage(service::Service::new(platform::native()))
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::set_enabled,
            commands::choose_directory,
            commands::add_share,
            commands::remove_share,
            commands::copy_address,
            commands::reveal_share,
            commands::open_settings,
            commands::hide_panel,
            commands::request_quit,
            desktop::startup_warning
        ])
        .setup(desktop::setup)
        .run(tauri::generate_context!())
        .expect("无法启动 SMB 共享助手");
}
