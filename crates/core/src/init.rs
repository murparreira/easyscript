//! A first project file for a project that has none, from what its dialogue already says:
//! one character per speaker, each tag with a kind guessed from its values, the functions
//! that set and read flags, the comment style of section headers. A draft to edit by hand.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use es_syntax::SyntaxKind::*;
use es_syntax::{SyntaxElement, SyntaxToken};
use walkdir::WalkDir;

use crate::lower::{lower, unquote};
use crate::model::{NodeKind, walk};
use crate::project::{LoadError, Project};
use crate::stats::stats;

/// Write the draft as TOML, with comments saying what was guessed and why.
pub fn draft(root: &Path) -> Result<String, LoadError> {
    // From the defaults: a project file already there does not shape the draft.
    let project = Project::with_config(root, crate::Config::default(), None)?;
    let script = lower(&project);
    let counts = stats(&script);
    let mut out = String::new();
    let w = &mut out;

    writeln!(
        w,
        "# easyscript project file, drafted by `easyscript init` from the dialogue files."
    )
    .ok();
    writeln!(
        w,
        "# Every value below is a guess from what the files say: check it, then edit freely."
    )
    .ok();
    writeln!(
        w,
        "# The format: https://github.com/murparreira/easyscript/blob/main/docs/project-file.md"
    )
    .ok();
    writeln!(w).ok();
    writeln!(w, "name = {}", q(&project_name(root))).ok();
    if let Some(header) = section_header(&project) {
        writeln!(
            w,
            "# Comments like `{}` open a section of labels.",
            header.0
        )
        .ok();
        writeln!(w, "section_header = {}", q_literal(&header.1)).ok();
    }
    writeln!(w).ok();

    writeln!(w, "[sources]").ok();
    writeln!(w, "dialogue = [\"**/*.dialogue\"]").ok();
    writeln!(w, "exclude = [\"addons/**\"]").ok();
    if root.join("project.godot").is_file() {
        writeln!(
            w,
            "# A Godot project: labels named in its scripts and scenes count as reached."
        )
        .ok();
        writeln!(w, "code = [\"**/*.gd\", \"**/*.tscn\", \"**/*.tres\"]").ok();
    } else {
        writeln!(
            w,
            "# The game's code, so that labels it names count as reached: code = [\"src/**/*.cs\"]"
        )
        .ok();
    }
    writeln!(w).ok();

    // Characters: one per speaker, most lines first.
    writeln!(
        w,
        "# One character per speaker name, most lines first. Join speakers that are the same"
    )
    .ok();
    writeln!(
        w,
        "# character into one `speakers` list, and add a `portrait`."
    )
    .ok();
    let mut ids = BTreeSet::new();
    for speaker in counts
        .speakers
        .iter()
        .filter(|s| !s.name.is_empty() && !s.name.contains("{{"))
    {
        let mut id = slug(&speaker.name);
        while !ids.insert(id.clone()) {
            id.push('_');
        }
        writeln!(w, "[[characters]]").ok();
        writeln!(w, "id = {}", q(&id)).ok();
        writeln!(w, "name = {}", q(&speaker.name)).ok();
        writeln!(w, "speakers = [{}]", q(&speaker.name)).ok();
        writeln!(w, "# {}", plural(speaker.count, "line")).ok();
        writeln!(w).ok();
    }

    // Tags.
    let usage = tag_usage(&script);
    let files = image_index(root);
    for (name, (lines, values)) in &usage {
        writeln!(w, "[tags.{}]", key(name)).ok();
        let distinct = values.len();
        let valued: usize = values.values().sum();
        if valued * 2 <= *lines {
            // Mostly written bare (`[#thought]`): a flag; the odd value is a slip.
            writeln!(w, "kind = \"flag\"").ok();
            if valued > 0 {
                writeln!(
                    w,
                    "# {} with a value, which a flag does not take",
                    plural(valued, "use")
                )
                .ok();
            }
        } else if let Some((pattern, found)) = asset_pattern(values, &files) {
            writeln!(w, "kind = \"asset\"").ok();
            writeln!(w, "path = {}", q(&pattern)).ok();
            if is_image(&pattern) {
                writeln!(w, "show = \"face\"").ok();
            }
            writeln!(w, "# {found} of its {distinct} values are files there.").ok();
        } else if distinct <= 12 {
            let mut ordered: Vec<(&String, &usize)> = values.iter().collect();
            ordered.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            let list: Vec<String> = ordered.iter().map(|(v, _)| q(v)).collect();
            writeln!(w, "kind = \"enum\"").ok();
            writeln!(w, "values = [{}]", list.join(", ")).ok();
        } else {
            writeln!(w, "kind = \"text\"").ok();
        }
        writeln!(w, "# on {}", plural(*lines, "line")).ok();
        writeln!(w).ok();
    }

    // Flags.
    let (set, read) = flag_functions(&project);
    writeln!(w, "[flags]").ok();
    if set.is_empty() && read.is_empty() {
        writeln!(
            w,
            "# No call with \"flag\" in its name was found. Name the functions that set and"
        )
        .ok();
        writeln!(
            w,
            "# read flags: set = [\"set_flag\"], read = [\"has_flag\"]."
        )
        .ok();
    } else {
        writeln!(w, "# Calls with \"flag\" in their name and a string first; by their verb, which set and which read.").ok();
    }
    writeln!(
        w,
        "set = [{}]",
        set.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
    )
    .ok();
    writeln!(
        w,
        "read = [{}]",
        read.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
    )
    .ok();
    writeln!(w).ok();
    writeln!(
        w,
        "# Still to add by hand: places, variants, authorship marks, documents. See the format."
    )
    .ok();
    Ok(out)
}

/// The game's name: Godot's `config/name`, else the folder's.
fn project_name(root: &Path) -> String {
    let godot = std::fs::read_to_string(root.join("project.godot")).unwrap_or_default();
    godot
        .lines()
        .find_map(|l| l.strip_prefix("config/name="))
        .and_then(|v| toml::from_str::<toml::Value>(&format!("v = {v}")).ok())
        .and_then(|t| t.get("v")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| folder_name(root))
}

fn folder_name(root: &Path) -> String {
    root.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Script".to_owned())
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// A TOML basic string.
fn q(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

/// A TOML literal string (no escapes), for regexes.
fn q_literal(text: &str) -> String {
    if text.contains('\'') {
        q(text)
    } else {
        format!("'{text}'")
    }
}

/// A TOML key, quoted when it has to be.
fn key(name: &str) -> String {
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && !name.is_empty()
    {
        name.to_owned()
    } else {
        q(name)
    }
}

/// A lowercase ASCII id: `Avó Ovelha` → `avo_ovelha`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().map(fold) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_owned();
    if out.is_empty() {
        "character".to_owned()
    } else {
        out
    }
}

/// Latin letters without their accents, enough for ids.
fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'Ñ' => 'N',
        other => other,
    }
}

/// The most common section-header comment style, if it repeats: (an example, a regex).
fn section_header(project: &Project) -> Option<(String, String)> {
    let mut styles: HashMap<char, (usize, String)> = HashMap::new();
    for file in &project.files {
        for token in comments(file.parse.syntax()) {
            let text = token.text().trim_start_matches('#').trim();
            let Some(first) = text.chars().next().filter(|c| matches!(c, '=' | '-' | '*')) else {
                continue;
            };
            let run = text.chars().take_while(|&c| c == first).count();
            let rest = text[run..].trim().trim_end_matches(first).trim();
            if run >= 3 && !rest.is_empty() {
                let entry = styles.entry(first).or_insert((0, token.text().to_owned()));
                entry.0 += 1;
            }
        }
    }
    let (style, (count, example)) = styles.into_iter().max_by_key(|(_, (n, _))| *n)?;
    let s = regex::escape(&style.to_string());
    (count >= 3).then(|| {
        (
            example,
            format!(r"^#\s*{s}{{3,}}\s*(?P<title>.*?)\s*{s}*\s*$"),
        )
    })
}

fn comments(root: es_syntax::SyntaxNode) -> impl Iterator<Item = SyntaxToken> {
    root.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|t| t.kind() == COMMENT && t.parent().is_some_and(|p| p.kind() == COMMENT_LINE))
}

/// Each tag: on how many lines, and its values with how often each is used (a tag never
/// given a value has none).
type TagUsage = BTreeMap<String, (usize, BTreeMap<String, usize>)>;

fn tag_usage(script: &crate::model::Script) -> TagUsage {
    let mut usage: TagUsage = BTreeMap::new();
    let mut add = |tags: &[crate::model::Tag]| {
        for tag in tags {
            let (lines, values) = usage.entry(tag.name.clone()).or_default();
            *lines += 1;
            if let Some(value) = &tag.value {
                *values.entry(value.clone()).or_default() += 1;
            }
        }
    };
    for file in &script.files {
        let bodies = std::iter::once(&file.preamble).chain(
            file.sections
                .iter()
                .flat_map(|s| &s.labels)
                .map(|l| &l.body),
        );
        for body in bodies {
            walk(body, &mut |node| match &node.kind {
                NodeKind::Line(line) => add(&line.tags),
                NodeKind::Menu { choices } => choices.iter().for_each(|c| add(&c.tags)),
                _ => {}
            });
        }
    }
    usage
}

/// Image, sound and scene files by stem: where each could live.
fn image_index(root: &Path) -> HashMap<String, Vec<(String, String)>> {
    let mut index: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'));
    for entry in walker
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
            continue;
        };
        if !matches!(
            ext.as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "svg"
                | "gif"
                | "ogg"
                | "wav"
                | "mp3"
                | "tscn"
                | "tres"
        ) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let dir = relative
            .parent()
            .map(|d| {
                d.components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        index.entry(stem).or_default().push((dir, ext));
    }
    index
}

/// A path pattern most of a tag's values are files in: (pattern, how many values match).
fn asset_pattern(
    values: &BTreeMap<String, usize>,
    files: &HashMap<String, Vec<(String, String)>>,
) -> Option<(String, usize)> {
    let mut places: HashMap<(&str, &str), usize> = HashMap::new();
    for value in values.keys() {
        for (dir, ext) in files.get(value).into_iter().flatten() {
            *places.entry((dir, ext)).or_default() += 1;
        }
    }
    let ((dir, ext), found) = places.into_iter().max_by_key(|(_, n)| *n)?;
    // Most values, and more than a couple: a real convention, not a coincidence.
    (found * 5 >= values.len() * 4 && found >= 2.min(values.len())).then(|| {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        (format!("{prefix}{{value}}.{ext}"), found)
    })
}

fn is_image(pattern: &str) -> bool {
    [".png", ".jpg", ".jpeg", ".webp", ".svg", ".gif"]
        .iter()
        .any(|e| pattern.ends_with(e))
}

/// Calls with "flag" in their name and a string first argument, split by their verb.
fn flag_functions(project: &Project) -> (Vec<String>, Vec<String>) {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for file in &project.files {
        for expr in file
            .parse
            .syntax()
            .descendants()
            .filter(|n| n.kind() == EXPR)
        {
            let tokens: Vec<SyntaxToken> = expr
                .descendants_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter(|t| t.kind() != WHITESPACE)
                .collect();
            for window in tokens.windows(3) {
                if let [name, open, arg] = window
                    && name.kind() == IDENT
                    && open.kind() == L_PAREN
                    && arg.kind() == STRING
                    && name.text().to_lowercase().contains("flag")
                    && !unquote(arg.text()).is_empty()
                {
                    names.insert(name.text().to_owned());
                }
            }
        }
    }
    let setter = |n: &str| {
        let n = n.to_lowercase();
        [
            "set", "add", "give", "raise", "mark", "enable", "unlock", "clear", "unset", "remove",
        ]
        .iter()
        .any(|v| n.starts_with(v))
    };
    let (set, read) = names.into_iter().partition(|n| setter(n));
    (set, read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(slug("Avó Ovelha"), "avo_ovelha");
        assert_eq!(slug("Professor Lobo-Guará"), "professor_lobo_guara");
        assert_eq!(slug("???"), "character");
    }
}
