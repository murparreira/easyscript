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
    GitHub::default().push(&root, "rewrite", "tok").unwrap();
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
    let error = GitHub::default().push(&root, "rewrite", "tok").unwrap_err();
    assert!(error.0.contains("commits you do not have"), "{error}");
}
