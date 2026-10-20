//! The script model: project, characters, scenes, lines, choices, conditions, effects
//! and tags, built from format syntax trees; the checks and stats over it.
//!
//! ```no_run
//! let project = dm_core::Project::load(std::path::Path::new("."), None)?;
//! let script = dm_core::lower(&project);
//! for problem in dm_core::check(&project, &script) {
//!     eprintln!("{problem}");
//! }
//! # Ok::<(), dm_core::LoadError>(())
//! ```

pub mod check;
pub mod config;
pub mod edit;
pub mod flow;
pub mod init;
mod lower;
pub mod model;
mod project;
pub mod project_file;
pub mod stats;
pub mod view;
pub mod workspace;

pub use check::{Diagnostic, Severity, check};
pub use config::Config;
pub use edit::{Edit, apply};
pub use lower::lower;
pub use model::Script;
pub use project::{LoadError, Project, SourceFile};
pub use stats::{Stats, stats};
pub use view::{ReadView, read_view};
pub use workspace::{SaveError, Workspace};

/// Version of the core, shown by the app and the CLI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
