// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dm_core::Workspace;
use dm_core::edit::Edit;
use dm_core::git::{self, GitStatus};
use dm_core::project_file::ConfigEdit;
use dm_core::workspace::{FileDiff, Session};
use dm_github::{Account, GitHub, PullRequest, Repo, SignIn};
use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Builder, Emitter, Manager, Runtime, State};

/// The open project (its working copies and edits), and the watcher that follows its files.
#[derive(Default)]
struct Opened(Arc<Mutex<Option<Open>>>);

struct Open {
    workspace: Workspace,
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
}

/// After an edit: the new session, and where the edited line starts now.
#[derive(Serialize)]
struct Edited {
    session: Session,
    at: usize,
}

/// Run `f` on the open workspace, off the main thread (loading and views take a moment).
async fn with_workspace<T: Send + 'static>(
    opened: &Opened,
    f: impl FnOnce(&mut Workspace) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let shared = opened.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = shared
            .lock()
            .map_err(|_| "the project is unusable after an error; open it again".to_owned())?;
        let open = guard.as_mut().ok_or("no project is open")?;
        f(&mut open.workspace)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Open a project folder: read it, and start following its files.
#[tauri::command]
async fn open_project<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    opened: State<'_, Opened>,
) -> Result<Session, String> {
    let root = PathBuf::from(path);
    let (workspace, session) = tauri::async_runtime::spawn_blocking({
        let root = root.clone();
        move || {
            let workspace = Workspace::open(&root, None).map_err(|e| e.to_string())?;
            let session = workspace.session();
            Ok::<_, String>((workspace, session))
        }
    })
    .await
    .map_err(|e| e.to_string())??;
    let watcher = watch(&root, app.clone()).map_err(|e| format!("cannot follow changes: {e}"))?;
    *opened
        .0
        .lock()
        .map_err(|_| "the app is unusable after an error")? = Some(Open {
        workspace,
        _watcher: watcher,
    });
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&format!("{} — dialogue-maker", session.view.title));
    }
    Ok(session)
}

/// Read the files again after `project-changed`; unsaved edits stay.
#[tauri::command]
async fn reload(opened: State<'_, Opened>) -> Result<Session, String> {
    with_workspace(&opened, |ws| {
        ws.refresh().map_err(|e| e.to_string())?;
        Ok(ws.session())
    })
    .await
}

/// Make edits to the project file's working copy, as one step.
#[tauri::command]
async fn edit_project_file(
    edits: Vec<ConfigEdit>,
    opened: State<'_, Opened>,
) -> Result<Session, String> {
    with_workspace(&opened, move |ws| {
        ws.edit_project_file(&edits).map_err(|e| e.to_string())?;
        Ok(ws.session())
    })
    .await
}

/// For a project with none: a draft project file from its dialogue, unsaved.
#[tauri::command]
async fn draft_project_file(opened: State<'_, Opened>) -> Result<Session, String> {
    with_workspace(&opened, |ws| {
        ws.draft_project_file().map_err(|e| e.to_string())?;
        Ok(ws.session())
    })
    .await
}

/// Make edits to one file's working copy, as one step.
#[tauri::command]
async fn edit(path: String, edits: Vec<Edit>, opened: State<'_, Opened>) -> Result<Edited, String> {
    with_workspace(&opened, move |ws| {
        let at = ws.edit_batch(&path, &edits).map_err(|e| e.to_string())?;
        Ok(Edited {
            session: ws.session(),
            at,
        })
    })
    .await
}

#[tauri::command]
async fn undo(opened: State<'_, Opened>) -> Result<Session, String> {
    with_workspace(&opened, |ws| {
        ws.undo();
        Ok(ws.session())
    })
    .await
}

#[tauri::command]
async fn redo(opened: State<'_, Opened>) -> Result<Session, String> {
    with_workspace(&opened, |ws| {
        ws.redo();
        Ok(ws.session())
    })
    .await
}

/// The unsaved edits, file by file.
#[tauri::command]
async fn diffs(opened: State<'_, Opened>) -> Result<Vec<FileDiff>, String> {
    with_workspace(&opened, |ws| Ok(ws.diffs())).await
}

/// Write the edited files; refused when one changed on disk.
#[tauri::command]
async fn save(opened: State<'_, Opened>) -> Result<Session, String> {
    with_workspace(&opened, |ws| {
        ws.save().map_err(|e| e.to_string())?;
        Ok(ws.session())
    })
    .await
}

/// Drop the unsaved edits and read the files again.
#[tauri::command]
async fn discard(opened: State<'_, Opened>) -> Result<Session, String> {
    with_workspace(&opened, |ws| {
        ws.discard().map_err(|e| e.to_string())?;
        Ok(ws.session())
    })
    .await
}

/// How the project's files stand in git; none when it is not in a repository.
#[tauri::command]
async fn git_status(opened: State<'_, Opened>) -> Result<Option<GitStatus>, String> {
    with_workspace(&opened, |ws| {
        git::status(ws.root(), &ws.owned_paths()).map_err(|e| e.to_string())
    })
    .await
}

/// Commit these project files as saved; refused while they have unsaved edits.
#[tauri::command]
async fn git_commit(
    paths: Vec<String>,
    message: String,
    opened: State<'_, Opened>,
) -> Result<String, String> {
    with_workspace(&opened, move |ws| {
        let unsaved: Vec<String> = ws
            .state()
            .changed
            .into_iter()
            .filter(|c| paths.contains(c))
            .collect();
        if !unsaved.is_empty() {
            return Err(format!(
                "save first: {} {} unsaved edits",
                unsaved.join(", "),
                if unsaved.len() == 1 { "has" } else { "have" }
            ));
        }
        let owned = ws.owned_paths();
        if let Some(other) = paths.iter().find(|p| !owned.contains(p)) {
            return Err(format!("{other} is not one of the project's files"));
        }
        git::commit(ws.root(), &paths, &message).map_err(|e| e.to_string())
    })
    .await
}

/// Make a branch, or switch to one, and read the project again from it. Refused while
/// there are unsaved edits.
#[tauri::command]
async fn git_branch(
    name: String,
    create: bool,
    opened: State<'_, Opened>,
) -> Result<Session, String> {
    with_workspace(&opened, move |ws| {
        if !ws.state().changed.is_empty() {
            return Err("save or discard your edits before changing branch".to_owned());
        }
        if create {
            git::create_branch(ws.root(), &name).map_err(|e| e.to_string())?;
        } else {
            git::switch_branch(ws.root(), &name).map_err(|e| e.to_string())?;
        }
        ws.refresh().map_err(|e| e.to_string())?;
        Ok(ws.session())
    })
    .await
}

/// The GitHub sign-in: the token (in the system's password store when there is one, else
/// only in memory for this session), who it signs in as, and a sign-in under way.
struct GitHubSession {
    client: GitHub,
    /// Keep the token in the system's password store (the tests keep it in memory).
    keep: bool,
    token: Mutex<Option<String>>,
    account: Mutex<Option<Account>>,
    /// A sign-in under way, and the flag that cancels it: each its own, so a sign-in
    /// cancelled and started again never finishes the old one.
    pending: Mutex<Option<(SignIn, Arc<AtomicBool>)>>,
}

const KEYRING_SERVICE: &str = "dialogue-maker";
const KEYRING_USER: &str = "github";

fn keyring_entry() -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()
}

impl GitHubSession {
    fn new(client: GitHub, keep: bool) -> Self {
        Self {
            client,
            keep,
            token: Mutex::default(),
            account: Mutex::default(),
            pending: Mutex::default(),
        }
    }

    fn store(&self) -> Option<keyring::Entry> {
        self.keep.then(keyring_entry).flatten()
    }

    /// The token: in memory, or read from the password store.
    fn token(&self) -> Option<String> {
        let mut token = self.token.lock().ok()?;
        if token.is_none() {
            *token = self.store().and_then(|e| e.get_password().ok());
        }
        token.clone()
    }

    fn forget(&self) {
        if let Ok(mut token) = self.token.lock() {
            *token = None;
        }
        if let Ok(mut account) = self.account.lock() {
            *account = None;
        }
        if let Some(entry) = self.store() {
            let _ = entry.delete_credential();
        }
    }
}

/// What the app knows about GitHub for the open project.
#[derive(Serialize)]
struct GitHubState {
    /// The repository `origin` points at, when it is on GitHub.
    repo: Option<Repo>,
    /// Who is signed in, when someone is.
    account: Option<Account>,
}

#[tauri::command]
async fn github_state(
    opened: State<'_, Opened>,
    github: State<'_, GitHubSession>,
) -> Result<GitHubState, String> {
    let repo = with_workspace(&opened, |ws| {
        dm_github::origin(ws.root()).map_err(|e| e.to_string())
    })
    .await?
    .and_then(|(_, repo)| repo);
    let known = github
        .account
        .lock()
        .map_err(|_| "unusable after an error")?
        .clone();
    let account = match (known, github.token()) {
        (Some(account), _) => Some(account),
        (None, Some(token)) => {
            let client = github.client.clone();
            let answer = tauri::async_runtime::spawn_blocking(move || client.account(&token))
                .await
                .map_err(|e| e.to_string())?;
            match answer {
                Ok(account) => {
                    *github
                        .account
                        .lock()
                        .map_err(|_| "unusable after an error")? = Some(account.clone());
                    Some(account)
                }
                Err(e) if e.0.contains("sign in again") => {
                    github.forget();
                    None
                }
                // Offline: say nothing about the account rather than fail.
                Err(_) => None,
            }
        }
        (None, None) => None,
    };
    Ok(GitHubState { repo, account })
}

/// Start signing in: the code to confirm on GitHub.
#[tauri::command]
async fn github_sign_in_start(github: State<'_, GitHubSession>) -> Result<SignIn, String> {
    let client = github.client.clone();
    let sign_in = tauri::async_runtime::spawn_blocking(move || client.start_sign_in())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mut pending = github
        .pending
        .lock()
        .map_err(|_| "unusable after an error")?;
    if let Some((_, cancel)) = pending.take() {
        cancel.store(true, Ordering::Relaxed);
    }
    *pending = Some((sign_in.clone(), Arc::default()));
    Ok(sign_in)
}

/// Wait for the code to be confirmed; keep the token, and say who signed in.
#[tauri::command]
async fn github_sign_in_finish(github: State<'_, GitHubSession>) -> Result<Account, String> {
    let (sign_in, cancel) = github
        .pending
        .lock()
        .map_err(|_| "unusable after an error")?
        .clone()
        .ok_or("no sign-in under way")?;
    let mine = cancel.clone();
    let client = github.client.clone();
    let (token, account) = tauri::async_runtime::spawn_blocking(move || {
        let token = client.finish_sign_in(&sign_in, &cancel)?;
        let account = client.account(&token)?;
        Ok::<_, dm_github::GitHubError>((token, account))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if let Some(entry) = github.store() {
        // Without a password store the sign-in lasts until the app closes.
        let _ = entry.set_password(&token);
    }
    *github.token.lock().map_err(|_| "unusable after an error")? = Some(token);
    *github
        .account
        .lock()
        .map_err(|_| "unusable after an error")? = Some(account.clone());
    let mut pending = github
        .pending
        .lock()
        .map_err(|_| "unusable after an error")?;
    if pending.as_ref().is_some_and(|(_, c)| Arc::ptr_eq(c, &mine)) {
        *pending = None;
    }
    Ok(account)
}

#[tauri::command]
fn github_sign_in_cancel(github: State<'_, GitHubSession>) {
    if let Some((_, cancel)) = github.pending.lock().ok().and_then(|mut p| p.take()) {
        cancel.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
fn github_sign_out(github: State<'_, GitHubSession>) {
    github.forget();
}

/// Push the branch and open its pull request (or find the one open). Refused with unsaved
/// edits, with project files not committed, and on the default branch.
#[tauri::command]
async fn github_pull_request(
    title: String,
    body: String,
    opened: State<'_, Opened>,
    github: State<'_, GitHubSession>,
) -> Result<PullRequest, String> {
    let token = github.token().ok_or("sign in to GitHub first")?;
    let (root, branch) = with_workspace(&opened, |ws| {
        if !ws.state().changed.is_empty() {
            return Err("save your edits and commit them first".to_owned());
        }
        let status = git::status(ws.root(), &ws.owned_paths())
            .map_err(|e| e.to_string())?
            .ok_or("the project is not in a git repository")?;
        if !status.files.is_empty() {
            return Err("commit your changes first: a pull request takes commits".to_owned());
        }
        let branch = status.branch.ok_or("check out a branch first")?;
        Ok((ws.root().to_owned(), branch))
    })
    .await?;
    let client = github.client.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (_, repo) = dm_github::origin(&root)?
            .ok_or_else(|| dm_github::GitHubError("the repository has no `origin`".into()))?;
        let repo =
            repo.ok_or_else(|| dm_github::GitHubError("`origin` is not on GitHub".into()))?;
        let base = client.default_branch(&token, &repo)?;
        if branch == base {
            return Err(dm_github::GitHubError(format!(
                "you are on {base}, where pull requests go: make a branch for your changes first"
            )));
        }
        client.push(&root, &branch, &token)?;
        let title = if title.trim().is_empty() {
            branch.clone()
        } else {
            title
        };
        client.pull_request(&token, &repo, &branch, &base, &title, &body)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
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

/// Emit `project-changed` when files under the root change.
fn watch<R: Runtime>(
    root: &Path,
    app: AppHandle<R>,
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

    app(
        Builder::default(),
        GitHubSession::new(GitHub::default(), true),
    )
    .run(tauri::generate_context!())
    .expect("error while running dialogue-maker");
}

/// The app's plugins, state and commands, on any runtime (the tests use a mock one, and a
/// fake GitHub).
fn app<R: Runtime>(builder: Builder<R>, github: GitHubSession) -> Builder<R> {
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Opened::default())
        .manage(github)
        .invoke_handler(tauri::generate_handler![
            open_project,
            reload,
            edit,
            edit_project_file,
            draft_project_file,
            undo,
            redo,
            diffs,
            save,
            discard,
            git_status,
            git_commit,
            git_branch,
            github_state,
            github_sign_in_start,
            github_sign_in_finish,
            github_sign_in_cancel,
            github_sign_out,
            github_pull_request,
            startup_project
        ])
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

    /// Call a command the way the frontend does, with the same JSON.
    fn call(
        webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        cmd: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        let request = tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: tauri::ipc::InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        tauri::test::get_ipc_response(webview, request).map(|body| body.deserialize().unwrap())
    }

    #[test]
    fn open_edit_save_through_the_commands() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.dialogue");
        std::fs::write(&file, "~ a\nKeeper: One.\n").unwrap();

        let app = app(
            tauri::test::mock_builder(),
            GitHubSession::new(GitHub::default(), false),
        )
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        let session = call(
            &webview,
            "open_project",
            serde_json::json!({ "path": dir.path() }),
        )
        .unwrap();
        assert_eq!(session["view"]["stats"]["lines"], 1);
        let line = &session["view"]["script"]["files"][0]["sections"][0]["labels"][0]["body"][0];
        assert_eq!(line["words"], "One.");
        let at = line["span"]["start"].clone();

        // The edit as the frontend sends it (its types come from the same Rust).
        let edited = call(
            &webview,
            "edit",
            serde_json::json!({ "path": "a.dialogue", "edits": [{ "op": "set_text", "at": at, "text": "Uno." }, { "op": "set_tag", "at": at, "name": "mood", "value": "calm" }] }),
        )
        .unwrap();
        assert_eq!(
            edited["session"]["state"]["changed"],
            serde_json::json!(["a.dialogue"])
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "~ a\nKeeper: One.\n",
            "not written before saving"
        );

        let diffs = call(&webview, "diffs", serde_json::json!({})).unwrap();
        assert_eq!(diffs[0]["path"], "a.dialogue");

        let saved = call(&webview, "save", serde_json::json!({})).unwrap();
        assert_eq!(saved["state"]["changed"], serde_json::json!([]));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "~ a\nKeeper: Uno. [#mood=calm]\n"
        );

        // Git: commits refuse unsaved edits, then take the saved file.
        let repo = git2::Repository::init(dir.path()).unwrap();
        repo.config()
            .unwrap()
            .set_str("user.name", "Writer")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "writer@example.com")
            .unwrap();
        let status = call(&webview, "git_status", serde_json::json!({})).unwrap();
        assert_eq!(status["files"][0]["path"], "a.dialogue");
        call(&webview, "edit", serde_json::json!({ "path": "a.dialogue", "edits": [{ "op": "set_text", "at": at, "text": "Dos." }] })).unwrap();
        let refused = call(
            &webview,
            "git_commit",
            serde_json::json!({ "paths": ["a.dialogue"], "message": "Two" }),
        )
        .unwrap_err();
        assert!(
            refused.as_str().unwrap().starts_with("save first"),
            "{refused}"
        );
        call(&webview, "save", serde_json::json!({})).unwrap();
        let id = call(
            &webview,
            "git_commit",
            serde_json::json!({ "paths": ["a.dialogue"], "message": "Two" }),
        )
        .unwrap();
        assert_eq!(id.as_str().unwrap().len(), 7);
        let status = call(&webview, "git_status", serde_json::json!({})).unwrap();
        assert_eq!(status["files"], serde_json::json!([]));
        assert!(status["head"].as_str().unwrap().ends_with("Two"));

        let error = call(&webview, "edit", serde_json::json!({ "path": "a.dialogue", "edits": [{ "op": "remove_line", "at": 1 }] })).unwrap_err();
        assert!(
            error.as_str().unwrap().contains("no line starts"),
            "{error}"
        );
    }

    /// A fake GitHub on localhost: the sign-in confirmed at the first poll, `team/game`
    /// with `main`, and pull requests opened as #7.
    fn fake_github() -> String {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = header.to_lowercase().strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap();
                    }
                }
                reader.read_exact(&mut vec![0; length]).unwrap();
                let mut parts = first.split_whitespace();
                let (method, path) = (parts.next().unwrap(), parts.next().unwrap());
                let (status, json) = match (method, path) {
                    ("POST", "/login/device/code") => (
                        200,
                        r#"{"device_code":"dev","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":60,"interval":1}"#,
                    ),
                    ("POST", "/login/oauth/access_token") => {
                        (200, r#"{"access_token":"tok","token_type":"bearer"}"#)
                    }
                    ("GET", "/user") => (200, r#"{"login":"writer","name":null}"#),
                    ("GET", "/repos/team/game") => (200, r#"{"default_branch":"main"}"#),
                    ("GET", p) if p.starts_with("/repos/team/game/pulls?") => (200, "[]"),
                    ("POST", "/repos/team/game/pulls") => (
                        201,
                        r#"{"number":7,"html_url":"https://github.com/team/game/pull/7"}"#,
                    ),
                    _ => (404, r#"{"message":"Not Found"}"#),
                };
                let reply = format!(
                    "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{json}",
                    json.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        base
    }

    #[test]
    fn sign_in_and_open_a_pull_request_through_the_commands() {
        let dir = tempfile::tempdir().unwrap();
        let hub = dir.path().join("hub");
        let origin = git2::Repository::init_bare(hub.join("team/game.git")).unwrap();
        let work = dir.path().join("work");
        let repo = git2::Repository::init_opts(
            &work,
            git2::RepositoryInitOptions::new().initial_head("main"),
        )
        .unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Writer").unwrap();
        config.set_str("user.email", "writer@example.com").unwrap();
        repo.remote("origin", "git@github.com:team/game.git")
            .unwrap();
        std::fs::write(work.join("a.dialogue"), "~ a\nKeeper: One.\n").unwrap();

        let base = fake_github();
        let mut client = GitHub::new(&base, &base);
        client.git = hub.to_str().unwrap().to_owned();
        let app = app(
            tauri::test::mock_builder(),
            GitHubSession::new(client, false),
        )
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let no_args = serde_json::json!({});
        let pull_request = serde_json::json!({ "title": "Rewrite", "body": "" });
        let session = call(
            &webview,
            "open_project",
            serde_json::json!({ "path": work }),
        )
        .unwrap();
        let at = session["view"]["script"]["files"][0]["sections"][0]["labels"][0]["body"][0]
            ["span"]["start"]
            .clone();
        call(
            &webview,
            "git_commit",
            serde_json::json!({ "paths": ["a.dialogue"], "message": "One" }),
        )
        .unwrap();

        let state = call(&webview, "github_state", no_args.clone()).unwrap();
        assert_eq!(
            state,
            serde_json::json!({ "repo": { "owner": "team", "name": "game" }, "account": null })
        );
        let refused = call(&webview, "github_pull_request", pull_request.clone()).unwrap_err();
        assert_eq!(refused, "sign in to GitHub first");

        let sign_in = call(&webview, "github_sign_in_start", no_args.clone()).unwrap();
        assert_eq!(sign_in["user_code"], "ABCD-1234");
        assert!(sign_in.get("device_code").is_none(), "kept in the app");
        let account = call(&webview, "github_sign_in_finish", no_args.clone()).unwrap();
        assert_eq!(account["login"], "writer");
        let state = call(&webview, "github_state", no_args.clone()).unwrap();
        assert_eq!(state["account"]["login"], "writer");

        // Pull requests come from a branch, with everything saved and committed.
        let refused = call(&webview, "github_pull_request", pull_request.clone()).unwrap_err();
        assert!(
            refused.as_str().unwrap().contains("you are on main"),
            "{refused}"
        );
        call(
            &webview,
            "git_branch",
            serde_json::json!({ "name": "rewrite", "create": true }),
        )
        .unwrap();
        call(&webview, "edit", serde_json::json!({ "path": "a.dialogue", "edits": [{ "op": "set_text", "at": at, "text": "Uno." }] })).unwrap();
        let refused = call(&webview, "github_pull_request", pull_request.clone()).unwrap_err();
        assert!(
            refused.as_str().unwrap().starts_with("save your edits"),
            "{refused}"
        );
        call(&webview, "save", no_args.clone()).unwrap();
        let refused = call(&webview, "github_pull_request", pull_request.clone()).unwrap_err();
        assert!(
            refused.as_str().unwrap().starts_with("commit your changes"),
            "{refused}"
        );
        call(
            &webview,
            "git_commit",
            serde_json::json!({ "paths": ["a.dialogue"], "message": "Uno" }),
        )
        .unwrap();

        let opened = call(&webview, "github_pull_request", pull_request).unwrap();
        assert_eq!(
            opened,
            serde_json::json!({ "number": 7, "url": "https://github.com/team/game/pull/7", "created": true })
        );
        assert_eq!(
            origin
                .find_reference("refs/heads/rewrite")
                .unwrap()
                .target(),
            repo.head().unwrap().target(),
            "pushed"
        );

        call(&webview, "github_sign_out", no_args.clone()).unwrap();
        let state = call(&webview, "github_state", no_args).unwrap();
        assert_eq!(state["account"], serde_json::Value::Null);
    }
}
