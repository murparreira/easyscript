// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use dm_core::{Project, ReadView, read_view};
use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tauri::{AppHandle, Emitter, Manager, State};

/// The open project, and the watcher that follows its files.
#[derive(Default)]
struct Opened(Mutex<Option<Watched>>);

struct Watched {
    root: PathBuf,
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
}

/// Open a project folder: read it, and start following its files.
#[tauri::command]
async fn open_project(
    path: String,
    app: AppHandle,
    opened: State<'_, Opened>,
) -> Result<ReadView, String> {
    let root = PathBuf::from(path);
    let view = load(root.clone()).await?;
    let watcher = watch(&root, app.clone()).map_err(|e| format!("cannot follow changes: {e}"))?;
    *opened.0.lock().expect("not poisoned") = Some(Watched {
        root,
        _watcher: watcher,
    });
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&format!("{} — dialogue-maker", view.title));
    }
    Ok(view)
}

/// A project folder given on the command line (`dm-app path/to/game`), if any.
#[tauri::command]
fn startup_project() -> Option<String> {
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.is_dir())
        .map(|p| p.canonicalize().unwrap_or(p).to_string_lossy().into_owned())
}

/// Read the open project again (after `project-changed`).
#[tauri::command]
async fn reload(opened: State<'_, Opened>) -> Result<ReadView, String> {
    let root = opened
        .0
        .lock()
        .expect("not poisoned")
        .as_ref()
        .map(|w| w.root.clone());
    load(root.ok_or("no project is open")?).await
}

/// Loading reads every file and makes thumbnails: off the main thread.
async fn load(root: PathBuf) -> Result<ReadView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let project = Project::load(&root, None).map_err(|e| e.to_string())?;
        Ok(read_view(&project))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Emit `project-changed` when files under the root change.
fn watch(
    root: &Path,
    app: AppHandle,
) -> notify_debouncer_full::notify::Result<Debouncer<RecommendedWatcher, RecommendedCache>> {
    let base = root.to_owned();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        None,
        move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            // Reading files is an event too (our own reload reads them all): only changes count.
            let changed = events
                .iter()
                .filter(|e| !e.kind.is_access())
                .any(|e| e.paths.iter().any(|p| matters(&base, p)));
            if changed {
                let _ = app.emit("project-changed", ());
            }
        },
    )?;
    debouncer.watch(root, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// Hidden folders (`.git`, `.godot`) change all the time and never hold the script; the
/// project's own `.dialogue-maker` folder does hold its documents.
fn matters(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !relative.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name.starts_with('.') && name != ".dialogue-maker"
    })
}

/// WebKitGTK's DMA-BUF renderer dies on the NVIDIA driver under Wayland
/// ("Error 71 (Protocol error) dispatching to Wayland display").
/// Fall back to shared-memory rendering there, unless the user chose already.
#[cfg(target_os = "linux")]
fn avoid_nvidia_dmabuf_crash() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_none() && Path::new("/proc/driver/nvidia").exists() {
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
        .plugin(tauri_plugin_dialog::init())
        .manage(Opened::default())
        .invoke_handler(tauri::generate_handler![
            open_project,
            reload,
            startup_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running dialogue-maker");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_folders_do_not_matter_but_documents_do() {
        let root = Path::new("/game");
        assert!(matters(root, Path::new("/game/src/a.dialogue")));
        assert!(matters(
            root,
            Path::new("/game/.dialogue-maker/walkthrough.md")
        ));
        assert!(!matters(root, Path::new("/game/.godot/imported/x.ctex")));
        assert!(!matters(root, Path::new("/game/.git/index")));
        assert!(!matters(root, Path::new("/elsewhere/a.dialogue")));
    }
}
