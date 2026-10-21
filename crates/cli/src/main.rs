//! `dialogue-maker check | stats | build | export | import | init`.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
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
    /// Write the read view as one HTML file that opens anywhere, offline.
    Build {
        #[command(flatten)]
        project: ProjectArgs,
        /// Where to write it.
        #[arg(short, long, default_value = "script.html")]
        output: PathBuf,
        /// Write the view's data as JSON instead of the page.
        #[arg(long)]
        json: bool,
        /// With --json: the view the app edits with (every face a line could pick), for
        /// working on the frontend.
        #[arg(long, hide = true, requires = "json")]
        editing: bool,
    },
    /// Write the script in another format: JSON for other tools, a translation table, voice
    /// scripts, a screenplay; or the dialogue files themselves as a ZIP.
    Export {
        /// What to write.
        format: ExportFormat,
        #[command(flatten)]
        project: ProjectArgs,
        /// Where to write it; standard output when not given. A translation table already
        /// there keeps its translations.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// translation: the locale the script is written in.
        #[arg(long, default_value = "en")]
        locale: String,
        /// translation: a column for this locale's translations (again for more).
        #[arg(long = "translate-to", value_name = "LOCALE")]
        translate_to: Vec<String>,
        /// voice: only this character (an id from the project file, or a speaker).
        #[arg(long)]
        character: Option<String>,
        /// voice, voice-csv, fountain: the language of the export's own words (en, pt-BR).
        #[arg(long, default_value = "en", value_parser = parse_language)]
        language: dm_formats::export::Language,
    },
    /// Convert a Twine (`.twee`), Yarn Spinner (`.yarn`) or Ink (`.ink`) script into a
    /// Dialogue Manager file (a `.dialogue` file is copied). What does not convert is kept
    /// as comments, and listed.
    Import {
        /// The script to convert.
        file: PathBuf,
        /// The `.dialogue` file to write (never over one that exists); standard output when
        /// not given.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Draft a project file (`dialogue-maker.toml`) for a project that has none, from what
    /// its dialogue says: characters, tags, flag functions, section headers.
    Init {
        /// The project's root folder.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Print the draft instead of writing it.
        #[arg(long)]
        stdout: bool,
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

fn parse_language(tag: &str) -> Result<dm_formats::export::Language, String> {
    dm_formats::export::Language::parse(tag).ok_or_else(|| format!("`{tag}`: en or pt-BR"))
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportFormat {
    /// The whole script as JSON, in a documented format (docs/formats).
    Json,
    /// Dialogue Manager's translation CSV, which Godot imports.
    Translation,
    /// Each character's lines by mood, as Markdown.
    Voice,
    /// Every spoken line as a recording sheet (CSV).
    VoiceCsv,
    /// A screenplay, in Fountain.
    Fountain,
    /// The dialogue files themselves, as a ZIP at their paths in the project (needs -o).
    Zip,
}

impl From<ExportFormat> for dm_formats::export::Format {
    fn from(f: ExportFormat) -> Self {
        use dm_formats::export::Format;
        match f {
            ExportFormat::Json => Format::Json,
            ExportFormat::Translation => Format::Translation,
            ExportFormat::Voice => Format::Voice,
            ExportFormat::VoiceCsv => Format::VoiceCsv,
            ExportFormat::Fountain => Format::Fountain,
            ExportFormat::Zip => unreachable!("written apart: it is not text"),
        }
    }
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
        Command::Build {
            project,
            output,
            json,
            editing,
        } => {
            let project = match project.load() {
                Ok(p) => p,
                Err(code) => return code,
            };
            let view = if editing {
                dm_core::view::editing_view(&project)
            } else {
                dm_core::read_view(&project)
            };
            let data = serde_json::to_string(&view).expect("serializable");
            let contents = if json {
                data
            } else if PAGE.is_empty() {
                eprintln!(
                    "dialogue-maker: this binary was built without the page; run `npm run build:page`, then build it again"
                );
                return ExitCode::from(2);
            } else {
                page(&data)
            };
            if let Err(e) = std::fs::write(&output, contents) {
                eprintln!("dialogue-maker: {}: {e}", output.display());
                return ExitCode::from(2);
            }
            eprintln!(
                "wrote {} ({} KB): {} labels, {} lines, {} problems",
                output.display(),
                std::fs::metadata(&output).map_or(0, |m| m.len() / 1024),
                view.stats.labels,
                view.stats.lines,
                view.problems.len()
            );
            ExitCode::SUCCESS
        }
        Command::Export {
            format,
            project,
            output,
            locale,
            translate_to,
            character,
            language,
        } => {
            let project = match project.load() {
                Ok(p) => p,
                Err(code) => return code,
            };
            if let ExportFormat::Zip = format {
                let Some(output) = output else {
                    eprintln!("dialogue-maker: a ZIP goes to a file: give it with -o");
                    return ExitCode::from(2);
                };
                let written = dm_formats::export::archive::dialogue_zip(&project, &[])
                    .and_then(|bytes| std::fs::write(&output, bytes).map_err(|e| e.to_string()));
                if let Err(e) = written {
                    eprintln!("dialogue-maker: {}: {e}", output.display());
                    return ExitCode::from(2);
                }
                eprintln!("wrote {}", output.display());
                return ExitCode::SUCCESS;
            }
            let script = dm_core::lower(&project);
            let existing = output
                .as_ref()
                .and_then(|o| std::fs::read_to_string(o).ok());
            let options = dm_formats::export::Options {
                locale: Some(locale),
                extra_locales: translate_to,
                character,
                language,
            };
            let text = match dm_formats::export::render(
                format.into(),
                &project,
                &script,
                &options,
                existing.as_deref(),
            ) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("dialogue-maker: {e}");
                    return ExitCode::from(2);
                }
            };
            match output {
                None => print(&text),
                Some(output) => {
                    if let Err(e) = std::fs::write(&output, &text) {
                        eprintln!("dialogue-maker: {}: {e}", output.display());
                        return ExitCode::from(2);
                    }
                    eprintln!("wrote {}", output.display());
                }
            }
            ExitCode::SUCCESS
        }
        Command::Import { file, output } => {
            let Some(source) = dm_formats::import::Source::of(&file) else {
                eprintln!(
                    "dialogue-maker: {}: not a .twee, .yarn, .ink or .dialogue file",
                    file.display()
                );
                return ExitCode::from(2);
            };
            let text = match std::fs::read_to_string(&file) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("dialogue-maker: {}: {e}", file.display());
                    return ExitCode::from(2);
                }
            };
            let imported = dm_formats::import::import(source, &text);
            for w in &imported.warnings {
                eprintln!("{}:{}: {}", file.display(), w.line, w.message);
            }
            match output {
                None => print(&imported.text),
                Some(output) => {
                    let written = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&output)
                        .and_then(|mut f| f.write_all(imported.text.as_bytes()));
                    if let Err(e) = written {
                        let why = if e.kind() == std::io::ErrorKind::AlreadyExists {
                            "it exists; it is left as it is".to_owned()
                        } else {
                            e.to_string()
                        };
                        eprintln!("dialogue-maker: {}: {why}", output.display());
                        return ExitCode::from(2);
                    }
                    eprintln!(
                        "wrote {} ({} {})",
                        output.display(),
                        imported.warnings.len(),
                        plural(imported.warnings.len(), "warning")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Command::Init { path, stdout } => {
            let target = path.join(dm_core::config::FILE_NAME);
            if !stdout && target.exists() {
                eprintln!(
                    "dialogue-maker: {} already exists; it is left as it is",
                    target.display()
                );
                return ExitCode::from(2);
            }
            let draft = match dm_core::init::draft(&path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("dialogue-maker: {e}");
                    return ExitCode::from(2);
                }
            };
            if stdout {
                print(&draft);
            } else if let Err(e) = std::fs::write(&target, draft) {
                eprintln!("dialogue-maker: {}: {e}", target.display());
                return ExitCode::from(2);
            } else {
                eprintln!("wrote {}: a draft to check and edit", target.display());
            }
            ExitCode::SUCCESS
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

/// The read view's page, as built by the frontend (empty when it was not built).
const PAGE: &str = include_str!(concat!(env!("OUT_DIR"), "/page.html"));

/// Where the page expects its data.
const DATA_SLOT: &str = "<script id=\"dm-view\" type=\"application/json\"></script>";

/// The page with the view's data in its slot. `<` is escaped so the data cannot end the
/// script element early.
fn page(data: &str) -> String {
    let data = data.replace('<', "\\u003c");
    PAGE.replacen(
        DATA_SLOT,
        &format!("<script id=\"dm-view\" type=\"application/json\">{data}</script>"),
        1,
    )
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
