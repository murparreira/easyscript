// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn core_version() -> &'static str {
    dm_core::VERSION
}

/// WebKitGTK's DMA-BUF renderer dies on the NVIDIA driver under Wayland
/// ("Error 71 (Protocol error) dispatching to Wayland display").
/// Fall back to shared-memory rendering there, unless the user chose already.
#[cfg(target_os = "linux")]
fn avoid_nvidia_dmabuf_crash() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_none() && std::path::Path::new("/proc/driver/nvidia").exists() {
        // SAFETY: called first thing in `main`, before any other thread exists.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var(VAR, "1");
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    avoid_nvidia_dmabuf_crash();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![core_version])
        .run(tauri::generate_context!())
        .expect("error while running dialogue-maker");
}
