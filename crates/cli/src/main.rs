//! `dialogue-maker check | stats`. `build` and `export` arrive with M3 and M8.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use dm_core::{Project, Severity};

/// A workbench for game scripts: reads a project's dialogue files where they live.
#[derive(Parser)]
#[command(name = "dialogue-maker", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report problems: unknown jump targets and speakers, missing portraits and assets,
    /// tags outside the project file, unreachable labels, broken document links.
    Check {
        #[command(flatten)]
        project: ProjectArgs,
        /// Print the problems as JSON.
        #[arg(long)]
        json: bool,
        /// Exit with an error on warnings too.
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Count labels, lines, choices, speakers and tags.
    Stats {
        #[command(flatten)]
        project: ProjectArgs,
        /// Print the counts as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args)]
struct ProjectArgs {
    /// The project's root folder.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// The project file, when it is not `dialogue-maker.toml` at the root.
    #[arg(long)]
    config: Option<PathBuf>,
}

impl ProjectArgs {
    fn load(&self) -> Result<Project, ExitCode> {
        Project::load(&self.path, self.config.as_deref()).map_err(|e| {
            eprintln!("dialogue-maker: {e}");
            ExitCode::from(2)
        })
    }
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Check {
            project,
            json,
            deny_warnings,
        } => {
            let project = match project.load() {
                Ok(p) => p,
                Err(code) => return code,
            };
            let script = dm_core::lower(&project);
            let problems = dm_core::check(&project, &script);
            let errors = problems
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            let warnings = problems.len() - errors;
            let mut out = String::new();
            if json {
                out = serde_json::to_string_pretty(&problems).expect("serializable") + "\n";
            } else {
                problems
                    .iter()
                    .for_each(|d| writeln!(out, "{d}").expect("to a string"));
                writeln!(
                    out,
                    "{} files: {errors} {}, {warnings} {}",
                    project.files.len(),
                    plural(errors, "error"),
                    plural(warnings, "warning")
                )
                .expect("to a string");
            }
            print(&out);
            ExitCode::from(u8::from(errors > 0 || (deny_warnings && warnings > 0)))
        }
        Command::Stats { project, json } => {
            let project = match project.load() {
                Ok(p) => p,
                Err(code) => return code,
            };
            let script = dm_core::lower(&project);
            let stats = dm_core::stats(&script);
            if json {
                print(&(serde_json::to_string_pretty(&stats).expect("serializable") + "\n"));
            } else {
                print(&format_stats(&project, &stats));
            }
            ExitCode::SUCCESS
        }
    }
}

/// Write to stdout; a closed pipe (`| head`) is not an error.
fn print(text: &str) {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}

fn format_stats(project: &Project, s: &dm_core::Stats) -> String {
    let mut out = String::new();
    if let Some(name) = &project.config.name {
        out.push_str(name);
        out.push('\n');
    }
    let rows = [
        (s.files, "files".to_owned()),
        (s.labels, "labels".to_owned()),
        (s.lines, "lines".to_owned()),
        (s.choices, format!("choices in {} menus", s.menus)),
        (s.branches, "branches".to_owned()),
        (s.jumps, "jumps".to_owned()),
        (s.effects, "effects".to_owned()),
        (s.words, "words".to_owned()),
        (s.flags_set, format!("flags set, {} read", s.flags_read)),
    ];
    for (count, what) in rows {
        writeln!(out, "{count:>7} {what}").expect("to a string");
    }
    let tables = [
        ("Lines by speaker", &s.speakers, "(narration)"),
        ("Tags", &s.tags, ""),
        ("Marks", &s.marks, ""),
    ];
    for (title, rows, empty) in tables.into_iter().filter(|(_, rows, _)| !rows.is_empty()) {
        writeln!(out, "\n{title}").expect("to a string");
        for row in rows {
            let name = if row.name.is_empty() {
                empty
            } else {
                &row.name
            };
            writeln!(out, "{:>7} {name}", row.count).expect("to a string");
        }
    }
    out
}
