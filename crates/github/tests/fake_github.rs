//! The GitHub client against a fake GitHub on localhost, and pushing to a local bare
//! repository.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use dm_github::{GitHub, Repo};

/// A request the fake received: method, path, body.
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

/// A fake GitHub answering with `answer(method, path, body)`: (status, JSON).
fn fake(answer: impl Fn(&str, &str, &str) -> (u16, String) + Send + 'static) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Arc::default();
    let log = seen.clone();
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
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let mut parts = first.split_whitespace();
            let (method, path) = (
                parts.next().unwrap().to_owned(),
                parts.next().unwrap().to_owned(),
            );
            let body = String::from_utf8(body).unwrap();
            let (status, json) = answer(&method, &path, &body);
            log.lock().unwrap().push((method, path, body));
            let reply = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{json}",
                json.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
        }
    });
    (base, seen)
}

#[test]
fn sign_in_with_the_device_flow() {
    let polls = Arc::new(Mutex::new(0));
    let counter = polls.clone();
    let (base, seen) = fake(move |method, path, body| match (method, path) {
        ("POST", "/login/device/code") => {
            assert!(body.contains("client_id=Ov23li9jPpUeZBeyVUIY") && body.contains("scope=repo"));
            (200, r#"{"device_code":"dev","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":60,"interval":1}"#.into())
        }
        ("POST", "/login/oauth/access_token") => {
            let mut n = counter.lock().unwrap();
            *n += 1;
            if *n == 1 {
                (200, r#"{"error":"authorization_pending"}"#.into())
            } else {
                (
                    200,
                    r#"{"access_token":"tok","token_type":"bearer"}"#.into(),
                )
            }
        }
        ("GET", "/user") => (
            200,
            r#"{"login":"writer","name":"The Writer","id":1}"#.into(),
        ),
        _ => (404, "{}".into()),
    });
    let github = GitHub::new(&base, &base);
    let sign_in = github.start_sign_in().unwrap();
    assert_eq!(sign_in.user_code, "ABCD-1234");
    let token = github
        .finish_sign_in(&sign_in, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(token, "tok");
    assert_eq!(*polls.lock().unwrap(), 2, "polled until confirmed");
    let account = github.account(&token).unwrap();
    assert_eq!(
        (account.login.as_str(), account.name.as_deref()),
        ("writer", Some("The Writer"))
    );
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .any(|(_, _, b)| b.contains("device_code=dev"))
    );
}

#[test]
fn a_refused_sign_in_says_so() {
    let (base, _) = fake(|_, path, _| {
        match path {
        "/login/device/code" => (200, r#"{"device_code":"d","user_code":"X","verification_uri":"u","expires_in":60,"interval":1}"#.into()),
        _ => (200, r#"{"error":"access_denied"}"#.into()),
    }
    });
    let github = GitHub::new(&base, &base);
    let sign_in = github.start_sign_in().unwrap();
    let error = github
        .finish_sign_in(&sign_in, &AtomicBool::new(false))
        .unwrap_err();
    assert!(error.0.contains("refused"), "{error}");
    assert!(
        github
            .finish_sign_in(&sign_in, &AtomicBool::new(true))
            .unwrap_err()
            .0
            .contains("cancelled")
    );
}

#[test]
fn open_or_find_a_pull_request() {
    let opened = Arc::new(Mutex::new(false));
    let state = opened.clone();
    let (base, seen) = fake(move |method, path, body| {
        let mut open = state.lock().unwrap();
        match (method, path) {
            ("GET", "/repos/team/game") => (200, r#"{"default_branch":"main"}"#.into()),
            ("GET", p) if p.starts_with("/repos/team/game/pulls?") => {
                // `owner:branch`, the colon possibly encoded.
                assert!(
                    p.contains("head=team:") || p.contains("head=team%3A"),
                    "{p}"
                );
                if *open && p.contains("rewrite") {
                    (
                        200,
                        r#"[{"number":7,"html_url":"https://github.com/team/game/pull/7"}]"#.into(),
                    )
                } else {
                    (200, "[]".into())
                }
            }
            ("POST", "/repos/team/game/pulls") => {
                let sent: serde_json::Value = serde_json::from_str(body).unwrap();
                if sent["head"] == "empty" {
                    return (422, r#"{"message":"Validation Failed","errors":[{"message":"No commits between main and empty"}]}"#.into());
                }
                assert_eq!(
                    (sent["base"].as_str(), sent["title"].as_str()),
                    (Some("main"), Some("Rewrite the keeper"))
                );
                *open = true;
                (
                    201,
                    r#"{"number":7,"html_url":"https://github.com/team/game/pull/7"}"#.into(),
                )
            }
            _ => (404, r#"{"message":"Not Found"}"#.into()),
        }
    });
    let github = GitHub::new(&base, &base);
    let repo = Repo {
        owner: "team".into(),
        name: "game".into(),
    };
    assert_eq!(github.default_branch("tok", &repo).unwrap(), "main");
    let first = github
        .pull_request(
            "tok",
            &repo,
            "rewrite",
            "main",
            "Rewrite the keeper",
            "Body",
        )
        .unwrap();
    assert!(first.created && first.number == 7);
    let again = github
        .pull_request(
            "tok",
            &repo,
            "rewrite",
            "main",
            "Rewrite the keeper",
            "Body",
        )
        .unwrap();
    assert!(!again.created && again.url.ends_with("/pull/7"));
    let error = github
        .pull_request("tok", &repo, "empty", "main", "x", "")
        .unwrap_err();
    assert!(
        error.0.contains("No commits between main and empty"),
        "{error}"
    );
    let missing = github
        .default_branch(
            "tok",
            &Repo {
                owner: "no".into(),
                name: "where".into(),
            },
        )
        .unwrap_err();
    assert!(missing.0.contains("not on GitHub"), "{missing}");
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|(_, path, _)| !path.is_empty())
    );
}

#[test]
fn fetch_brings_origins_branches_and_drops_deleted_ones() {
    use git2::{Repository, Signature};
    let dir = tempfile::tempdir().unwrap();
    let bare = Repository::init_bare(dir.path().join("origin.git")).unwrap();
    let sig = Signature::now("Teammate", "t@example.com").unwrap();
    let blob = bare.blob(b"~ a\n").unwrap();
    let mut tree = bare.treebuilder(None).unwrap();
    tree.insert("a.dialogue", blob, 0o100644).unwrap();
    let tree = bare.find_tree(tree.write().unwrap()).unwrap();
    let first = bare
        .commit(Some("refs/heads/main"), &sig, &sig, "start", &tree, &[])
        .unwrap();
    bare.reference("refs/heads/draft", first, true, "").unwrap();
    bare.set_head("refs/heads/main").unwrap();
    let work = Repository::init(dir.path().join("work")).unwrap();
    work.remote("origin", dir.path().join("origin.git").to_str().unwrap())
        .unwrap();
    let root = dir.path().join("work");

    GitHub::default().fetch(&root, None).unwrap();
    let tip = |name: &str| {
        work.find_reference(name)
            .ok()
            .and_then(|r| r.peel_to_commit().ok())
            .map(|c| c.id())
    };
    assert_eq!(tip("refs/remotes/origin/main"), Some(first));
    assert_eq!(tip("refs/remotes/origin/draft"), Some(first));
    assert_eq!(
        work.find_reference("refs/remotes/origin/HEAD")
            .unwrap()
            .symbolic_target()
            .ok()
            .flatten(),
        Some("refs/remotes/origin/main"),
        "the shared branch is known"
    );
    assert!(
        work.head().is_err(),
        "no branch of the writer's was touched"
    );

    bare.find_reference("refs/heads/draft")
        .unwrap()
        .delete()
        .unwrap();
    GitHub::default().fetch(&root, None).unwrap();
    assert_eq!(tip("refs/remotes/origin/draft"), None, "deleted on origin");
    assert_eq!(tip("refs/remotes/origin/HEAD"), Some(first));
}

#[test]
fn push_fast_forwards_only() {
    use git2::{Repository, Signature};
    let dir = tempfile::tempdir().unwrap();
    let bare = Repository::init_bare(dir.path().join("origin.git")).unwrap();
    let work = Repository::init(dir.path().join("work")).unwrap();
    work.remote("origin", dir.path().join("origin.git").to_str().unwrap())
        .unwrap();
    let sig = Signature::now("Writer", "w@example.com").unwrap();
    let commit = |repo: &Repository, file: &str, text: &str, refname: &str| {
        let blob = repo.blob(text.as_bytes()).unwrap();
        let mut tree = repo.treebuilder(None).unwrap();
        tree.insert(file, blob, 0o100644).unwrap();
        let tree = repo.find_tree(tree.write().unwrap()).unwrap();
        let parent = repo
            .find_reference(refname)
            .ok()
            .and_then(|r| r.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some(refname), &sig, &sig, text, &tree, &parents)
            .unwrap()
    };
    let first = commit(&work, "a.dialogue", "one", "refs/heads/rewrite");
    let root = dir.path().join("work");
    GitHub::default()
        .push(&root, "rewrite", Some("tok"))
        .unwrap();
    assert_eq!(
        bare.find_reference("refs/heads/rewrite").unwrap().target(),
        Some(first)
    );
    assert_eq!(
        work.find_reference("refs/remotes/origin/rewrite")
            .unwrap()
            .target(),
        Some(first)
    );

    // Someone else pushed to the branch: no force, a clear refusal.
    commit(&bare, "a.dialogue", "theirs", "refs/heads/rewrite");
    commit(&work, "a.dialogue", "mine", "refs/heads/rewrite");
    let error = GitHub::default()
        .push(&root, "rewrite", Some("tok"))
        .unwrap_err();
    assert!(error.0.contains("commits you do not have"), "{error}");
}

#[test]
fn list_repositories_the_account_can_push_to() {
    let (base, seen) = fake(|_, path, _| {
        if path.ends_with("&page=1") {
            let mut repos: Vec<String> = (0..100)
                .map(|i| {
                    format!(
                        r#"{{"name":"r{i}","owner":{{"login":"team"}},"private":true,"description":"","pushed_at":null,"permissions":{{"push":{}}}}}"#,
                        i != 3
                    )
                })
                .collect();
            repos[0] = r#"{"name":"game","owner":{"login":"team"},"private":false,"description":"The game","pushed_at":"2026-10-19T10:00:00Z","permissions":{"push":true}}"#.into();
            (200, format!("[{}]", repos.join(",")))
        } else {
            (
                200,
                r#"[{"name":"last","owner":{"login":"writer"},"private":false,"description":null,"pushed_at":null}]"#
                    .into(),
            )
        }
    });
    let repos = GitHub::new(&base, &base).repositories("tok").unwrap();
    assert_eq!(repos.len(), 100, "99 of the first page, one of the second");
    assert_eq!(repos[0].name, "game");
    assert_eq!(repos[0].description.as_deref(), Some("The game"));
    assert!(
        repos.iter().all(|r| r.name != "r3"),
        "read-only ones are left out"
    );
    assert_eq!(repos.last().unwrap().owner, "writer");
    assert!(
        repos[1].description.is_none(),
        "an empty description is none"
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn clone_and_link() {
    use git2::{Repository, Signature};
    let dir = tempfile::tempdir().unwrap();
    let hub = dir.path().join("hub");
    let bare = Repository::init_bare(hub.join("team/game.git")).unwrap();
    let sig = Signature::now("Writer", "w@example.com").unwrap();
    let blob = bare.blob(b"~ start\nHi.\n").unwrap();
    let mut tree = bare.treebuilder(None).unwrap();
    tree.insert("a.dialogue", blob, 0o100644).unwrap();
    let tree = bare.find_tree(tree.write().unwrap()).unwrap();
    bare.commit(Some("refs/heads/main"), &sig, &sig, "first", &tree, &[])
        .unwrap();
    bare.set_head("refs/heads/main").unwrap();

    let mut github = GitHub::default();
    // Over file://, so the clone reports its progress as a download does.
    // `file:///C:/...` on Windows, `file:///tmp/...` elsewhere.
    let hub_path = hub.to_str().unwrap().replace('\\', "/");
    github.git = format!("file:///{}", hub_path.trim_start_matches('/'));
    let repo = Repo {
        owner: "team".into(),
        name: "game".into(),
    };
    let into = dir.path().join("work/game");
    let mut calls = 0;
    github
        .clone_repo(&repo, &into, "tok", &mut |_, _| {
            calls += 1;
            true
        })
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(into.join("a.dialogue")).unwrap(),
        "~ start\nHi.\n"
    );
    let (url, on_github) = dm_github::origin(&into).unwrap().unwrap();
    assert_eq!(url, "https://github.com/team/game.git");
    assert_eq!(on_github, Some(repo.clone()));

    // Not over files that are there.
    let error = github
        .clone_repo(&repo, &into, "tok", &mut |_, _| true)
        .unwrap_err();
    assert!(error.0.contains("not an empty folder"), "{error}");

    assert!(calls > 0, "progress is reported");

    // Stopped, or failed: nothing stays.
    let other = dir.path().join("stopped");
    let error = github
        .clone_repo(&repo, &other, "tok", &mut |_, _| false)
        .unwrap_err();
    assert_eq!(error.0, "clone stopped");
    assert!(!other.exists());
    let missing = Repo {
        owner: "team".into(),
        name: "missing".into(),
    };
    std::fs::create_dir(&other).unwrap();
    let error = github
        .clone_repo(&missing, &other, "tok", &mut |_, _| true)
        .unwrap_err();
    assert!(error.0.starts_with("cannot clone team/missing"), "{error}");
    assert!(other.exists() && std::fs::read_dir(&other).unwrap().next().is_none());

    // Linking: only a repository with no origin.
    let error = dm_github::link(&into, &repo).unwrap_err();
    assert!(error.0.contains("has an `origin` already"), "{error}");
    let plain = dir.path().join("plain");
    Repository::init(&plain).unwrap();
    dm_github::link(&plain, &repo).unwrap();
    assert_eq!(dm_github::origin(&plain).unwrap().unwrap().1, Some(repo));
    let error = dm_github::link(
        dir.path(),
        &Repo {
            owner: "a".into(),
            name: "b".into(),
        },
    )
    .unwrap_err();
    assert!(error.0.contains("not in a git repository"), "{error}");
}
