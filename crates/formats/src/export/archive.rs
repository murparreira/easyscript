//! The project's dialogue files as a ZIP, at their paths in the project: unzipped over the
//! game's folder, it puts each file where it was.

use std::io::{Cursor, Write};

use es_core::Project;
use zip::write::SimpleFileOptions;

/// The dialogue files at `paths` (all of them when empty) as a ZIP, with their text as it
/// is in `project` (unsaved edits included, in the app).
pub fn dialogue_zip(project: &Project, paths: &[String]) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut wrote = 0;
    for file in &project.files {
        if !paths.is_empty() && !paths.contains(&file.path) {
            continue;
        }
        zip.start_file(file.path.as_str(), options)
            .map_err(|e| format!("{}: {e}", file.path))?;
        zip.write_all(file.text.as_bytes())
            .map_err(|e| format!("{}: {e}", file.path))?;
        wrote += 1;
    }
    if wrote == 0 {
        return Err("none of those files is one of the project's dialogue files".into());
    }
    let out = zip.finish().map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}
