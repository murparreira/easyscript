//! A real project loads, lowers and checks without unreadable text. The project's path
//! comes from `EASYSCRIPT_PROJECT` (relative to the workspace root) and its project
//! file, when it is not at the project root, from `EASYSCRIPT_CONFIG`. Without them
//! the test does nothing; CI never has them.
//!
//! `cargo test -p es-core --test real_project -- --nocapture` also prints the counts.

use std::path::{Path, PathBuf};

use es_core::{Project, check, lower, stats};

fn workspace_path(var: &str) -> Option<PathBuf> {
    let value = std::env::var_os(var)?;
    Some(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(value),
    )
}

#[test]
fn real_project_checks() {
    let Some(root) = workspace_path("EASYSCRIPT_PROJECT") else {
        eprintln!("skipped: EASYSCRIPT_PROJECT is not set");
        return;
    };
    let config = workspace_path("EASYSCRIPT_CONFIG");
    let project = Project::load(&root, config.as_deref()).expect("the project loads");
    let script = lower(&project);
    let problems = check(&project, &script);
    for problem in &problems {
        eprintln!("{problem}");
    }
    let unreadable: Vec<_> = problems
        .iter()
        .filter(|d| matches!(d.code, "syntax" | "unreadable-file"))
        .collect();
    assert!(unreadable.is_empty(), "unreadable text: {unreadable:#?}");

    let stats = stats(&script);
    eprintln!(
        "{} files, {} labels, {} lines, {} choices in {} menus",
        stats.files, stats.labels, stats.lines, stats.choices, stats.menus
    );
    assert!(stats.lines > 0);
}
