//! GiDuView - Native macOS UltraViewer-compatible Remote Desktop.

pub mod audio;
pub mod chat;
pub mod client;
pub mod clipboard;
pub mod command;
pub mod console;
pub mod control;
pub mod cursor;
pub mod direct;
pub mod files;
pub mod host;
pub mod input;
pub mod messages;
pub mod platform;
pub mod profile;
pub mod screen;
pub mod tauri_commands;
pub mod tls;
pub mod wire;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(tauri_commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            tauri_commands::get_system_permissions,
            tauri_commands::request_permissions,
            tauri_commands::get_host_info,
            tauri_commands::start_host,
            tauri_commands::stop_host,
            tauri_commands::connect_remote,
            tauri_commands::disconnect_remote,
            tauri_commands::send_mouse_move,
            tauri_commands::send_mouse_button,
            tauri_commands::send_mouse_wheel,
            tauri_commands::send_key_event,
            tauri_commands::set_view_only,
            tauri_commands::send_special_key,
            tauri_commands::get_recent_partners,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GiDuView application");
}
