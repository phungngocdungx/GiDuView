//! Tauri 2.0 command handlers and session manager for GiDuView.
//! Bridges the frontend UI with the UltraViewer Rust protocol engine,
//! providing live screen streaming, input forwarding, chat, and hosting.

use crate::client::{self, ClientEvent, ClientSession, ConnectOptions};
use crate::input::{self, KeyCode};
use crate::platform::macos::permissions::{
    check_permissions, open_input_settings, open_screen_settings, request_screen_permission,
    MacOSPermissions,
};
use crate::profile::Profile;
use crate::wire::Result;
use image::{ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct AppState {
    pub client: Arc<Mutex<Option<ClientSession>>>,
    pub encoder: Arc<Mutex<input::Encoder>>,
    pub view_only: Arc<AtomicBool>,
    pub host_active: Arc<AtomicBool>,
    pub host_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteConnectOptions {
    pub view_only: bool,
    pub clipboard: bool,
    pub quality: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectedPayload {
    pub width: u32,
    pub height: u32,
    pub partner_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FramePayload {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub data: String, // Base64 JPEG or PNG
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostInfo {
    pub id: String,
    pub password: String,
    pub is_hosting: bool,
}

#[tauri::command]
pub async fn get_system_permissions() -> std::result::Result<MacOSPermissions, String> {
    Ok(check_permissions())
}

#[tauri::command]
pub async fn request_permissions(permission: String) -> std::result::Result<bool, String> {
    match permission.as_str() {
        "screen" => {
            let res = request_screen_permission();
            if !res {
                open_screen_settings();
            }
            Ok(res)
        }
        "accessibility" => {
            open_input_settings();
            Ok(true)
        }
        _ => Err("Invalid permission type".to_string()),
    }
}

#[tauri::command]
pub async fn get_host_info(state: State<'_, AppState>) -> std::result::Result<HostInfo, String> {
    let profile = Profile::load_or_create_host().map_err(|e| e.to_string())?;
    let is_hosting = state.host_active.load(Ordering::SeqCst);
    Ok(HostInfo {
        id: profile.registration.id.to_string(),
        password: profile.registration.password,
        is_hosting,
    })
}

#[tauri::command]
pub async fn start_host(
    state: State<'_, AppState>,
    app: AppHandle,
) -> std::result::Result<HostInfo, String> {
    if state.host_active.load(Ordering::SeqCst) {
        return get_host_info(state).await;
    }

    let profile = Profile::load_or_create_host().map_err(|e| e.to_string())?;
    let info = HostInfo {
        id: profile.registration.id.to_string(),
        password: profile.registration.password.clone(),
        is_hosting: true,
    };

    state.host_active.store(true, Ordering::SeqCst);
    let host_active = state.host_active.clone();
    let app_handle = app.clone();

    let handle = thread::spawn(move || {
        let opts = crate::host::Options {
            auto_approve: Some(true),
            selection: crate::host::layout::Selection::All,
            clipboard: true,
            backend: crate::host::BackendChoice::Native,
            download_dir: None,
        };

        let _ = app_handle.emit("host-status", "Hosting active. Ready for connections.");
        if let Err(e) = crate::host::run_with(profile, opts) {
            let _ = app_handle.emit("host-status", format!("Hosting stopped: {e}"));
        }
        host_active.store(false, Ordering::SeqCst);
        let _ = app_handle.emit("host-stopped", ());
    });

    *state.host_handle.lock().unwrap() = Some(handle);
    Ok(info)
}

#[tauri::command]
pub async fn stop_host(state: State<'_, AppState>) -> std::result::Result<(), String> {
    state.host_active.store(false, Ordering::SeqCst);
    if let Some(_handle) = state.host_handle.lock().unwrap().take() {
        // Host thread terminates
    }
    Ok(())
}

#[tauri::command]
pub async fn connect_remote(
    id: String,
    password: String,
    options: Option<RemoteConnectOptions>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> std::result::Result<(), String> {
    // Disconnect any existing session
    disconnect_remote(state.clone()).await?;

    let profile = Profile::load_or_create().map_err(|e| e.to_string())?;
    let view_only = options.as_ref().map(|o| o.view_only).unwrap_or(false);
    state.view_only.store(view_only, Ordering::SeqCst);

    let connect_opts = ConnectOptions {
        clipboard: options.as_ref().map(|o| o.clipboard).unwrap_or(true),
        local_screen: Some((1920, 1080)),
        ..Default::default()
    };

    let app_handle = app.clone();
    let (tx_event, rx_event) = std::sync::mpsc::channel::<ClientEvent>();

    let callback_tx = tx_event.clone();
    let id_clone = id.clone();
    let pw_clone = password.clone();

    let session_res = thread::spawn(move || {
        client::connect_with(
            &profile,
            &id_clone,
            &pw_clone,
            connect_opts,
            Box::new(move |ev| {
                let _ = callback_tx.send(ev);
            }),
        )
    })
    .join()
    .map_err(|_| "Failed to start client session thread".to_string())?
    .map_err(|e| e.to_string())?;

    let session_screen = session_res.screen.clone();
    let partner_version = session_res.partner_version();
    *state.client.lock().unwrap() = Some(session_res);

    let supports_binary = input::supports_binary_keyboard(&partner_version);
    *state.encoder.lock().unwrap() = input::Encoder::new(supports_binary);

    // Spawn event dispatcher thread
    thread::spawn(move || {
        let mut last_frame_instant = std::time::Instant::now();
        for ev in rx_event {
            match ev {
                ClientEvent::Status(status) => {
                    let _ = app_handle.emit("session-status", status);
                }
                ClientEvent::Connected { width, height } => {
                    let _ = app_handle.emit(
                        "session-connected",
                        ConnectedPayload {
                            width,
                            height,
                            partner_version: partner_version.clone(),
                        },
                    );
                }
                ClientEvent::FrameReady => {
                    // Throttle frame encoding to max 60 FPS
                    if last_frame_instant.elapsed() >= Duration::from_millis(16) {
                        last_frame_instant = std::time::Instant::now();
                        let guard = session_screen.lock().unwrap();
                        if let Some(renderer) = guard.as_ref() {
                            let width = renderer.width();
                            let height = renderer.height();
                            let pixels = renderer.pixels(); // &[u32] ARGB
                            if let Ok(jpeg_base64) = encode_pixels_to_jpeg(pixels, width, height, 75) {
                                let _ = app_handle.emit(
                                    "session-frame",
                                    FramePayload {
                                        width,
                                        height,
                                        format: "jpeg".to_string(),
                                        data: jpeg_base64,
                                    },
                                );
                            }
                        }
                    }
                }
                ClientEvent::Cursor(cursor) => {
                    let _ = app_handle.emit("session-cursor", format!("{cursor:?}"));
                }
                ClientEvent::ViewOnly(vo) => {
                    let _ = app_handle.emit("session-view-only", vo);
                }
                ClientEvent::Chat(chat_event) => {
                    let _ = app_handle.emit("session-chat", format!("{chat_event:?}"));
                }
                ClientEvent::Closed(reason) => {
                    let _ = app_handle.emit("session-closed", reason);
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn disconnect_remote(state: State<'_, AppState>) -> std::result::Result<(), String> {
    if let Some(session) = state.client.lock().unwrap().take() {
        session.close();
    }
    Ok(())
}

#[tauri::command]
pub async fn send_mouse_move(
    x: i32,
    y: i32,
    wparam: i32,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    if state.view_only.load(Ordering::SeqCst) {
        return Ok(());
    }
    let guard = state.client.lock().unwrap();
    if let Some(session) = guard.as_ref() {
        let payload = input::mouse_move(wparam, x, y);
        session.send_input(payload);
    }
    Ok(())
}

#[tauri::command]
pub async fn send_mouse_button(
    msg: u32,
    wparam: i32,
    lparam: i32,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    if state.view_only.load(Ordering::SeqCst) {
        return Ok(());
    }
    let guard = state.client.lock().unwrap();
    if let Some(session) = guard.as_ref() {
        let payload = input::mouse_button(msg, wparam, lparam);
        session.send_input(payload);
    }
    Ok(())
}

#[tauri::command]
pub async fn send_mouse_wheel(
    delta: i16,
    key_flags: u16,
    x: i32,
    y: i32,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    if state.view_only.load(Ordering::SeqCst) {
        return Ok(());
    }
    let guard = state.client.lock().unwrap();
    if let Some(session) = guard.as_ref() {
        let wparam = input::wheel_wparam(delta, key_flags);
        let lparam = input::packed_position(x as i16, y as i16);
        let payload = input::mouse_wheel(wparam, lparam);
        session.send_input(payload);
    }
    Ok(())
}

#[tauri::command]
pub async fn send_key_event(
    code: String,
    pressed: bool,
    text: Option<String>,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    if state.view_only.load(Ordering::SeqCst) {
        return Ok(());
    }
    let guard = state.client.lock().unwrap();
    if let Some(session) = guard.as_ref() {
        let key_code = KeyCode::from_name(&code).unwrap_or(KeyCode::Other);
        let mut enc = state.encoder.lock().unwrap();
        let payloads = enc.key(key_code, pressed, text.as_deref());
        for p in payloads {
            session.send_input(p);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_view_only(
    view_only: bool,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    state.view_only.store(view_only, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn send_special_key(
    action: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    if state.view_only.load(Ordering::SeqCst) {
        return Ok(());
    }
    let guard = state.client.lock().unwrap();
    if let Some(session) = guard.as_ref() {
        match action.as_str() {
            "ctrl_alt_del" => {
                session.send_input(b"41|||1".to_vec());
            }
            "win_key" => {
                let mut enc = state.encoder.lock().unwrap();
                for p in enc.key(KeyCode::SuperLeft, true, None) {
                    session.send_input(p);
                }
                for p in enc.key(KeyCode::SuperLeft, false, None) {
                    session.send_input(p);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_recent_partners() -> std::result::Result<Vec<String>, String> {
    Ok(vec![])
}

/// Helper function to convert ARGB u32 pixels to Base64 JPEG.
fn encode_pixels_to_jpeg(
    pixels: &[u32],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<String> {
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for &p in pixels {
        let r = ((p >> 16) & 0xff) as u8;
        let g = ((p >> 8) & 0xff) as u8;
        let b = (p & 0xff) as u8;
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }

    let mut jpeg_bytes = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, quality);
    encoder
        .write_image(&rgb, width, height, ColorType::Rgb8.into())
        .map_err(|e| format!("JPEG encode error: {e}"))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes))
}
