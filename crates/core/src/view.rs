//! What the read view shows: the script with everything around it the views need (places,
//! characters, tags, flags, documents, faces, counts, problems), in one serializable value.
//! The app and the static page render the same value.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::path::Path;

use base64::Engine as _;
use serde::Serialize;
use ts_rs::TS;

use crate::check::{Diagnostic, check};
use crate::config::TagKind;
use crate::lower::lower;
use crate::model::{FlagAccess, Script};
use crate::project::Project;
use crate::stats::{Stats, stats};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReadView {
    pub title: String,
    pub script: Script,
    pub places: Vec<PlaceView>,
    pub characters: Vec<CharacterView>,
    pub tags: Vec<TagView>,
    pub variants: Vec<VariantView>,
    pub flags: Vec<FlagView>,
    /// Functions whose first string argument is a flag, so conditions can show flags.
    pub flag_functions: FlagFunctions,
    pub documents: Vec<DocumentView>,
    /// Small versions of faces and portraits, by their path in the project, as data URIs.
    pub images: BTreeMap<String, String>,
    pub stats: Stats,
    pub problems: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PlaceView {
    pub id: String,
    pub name: String,
    pub group: Option<String>,
    pub files: Vec<String>,
    pub labels: Vec<String>,
    pub exclude_labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CharacterView {
    pub id: String,
    pub name: String,
    pub speakers: Vec<String>,
    /// A key into [`ReadView::images`].
    pub portrait: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TagView {
    pub name: String,
    pub kind: TagKindView,
    pub values: Vec<String>,
    /// For `asset` tags: where the file is, with `{value}`.
    pub path: Option<String>,
    pub show: Option<String>,
    pub description: Option<String>,
    /// For `asset` tags, in a view made for editing: the values whose files exist.
    pub available: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TagKindView {
    Flag,
    Enum,
    Asset,
    Text,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct VariantView {
    pub name: String,
    pub condition: String,
    pub if_true: String,
    pub if_false: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FlagView {
    pub flag: String,
    pub meaning: Option<String>,
    /// Labels that set it, in script order.
    pub set_by: Vec<String>,
    /// Labels that read it, in script order.
    pub read_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FlagFunctions {
    pub set: Vec<String>,
    pub read: Vec<String>,
    /// `match` subjects whose `when` values are flags.
    pub match_subjects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DocumentView {
    pub title: String,
    pub path: String,
    /// Markdown; `[[label]]` links into the script.
    pub text: String,
}

/// Size of the longer side of a thumbnail, in pixels (twice the largest avatar).
const THUMBNAIL: u32 = 128;

/// Load everything the read view needs from a project.
pub fn read_view(project: &Project) -> ReadView {
    build(project, false)
}

/// The read view with what editing needs too: every face a line could pick.
pub fn editing_view(project: &Project) -> ReadView {
    build(project, true)
}

fn build(project: &Project, editing: bool) -> ReadView {
    let config = &project.config;
    let available: BTreeMap<&String, Vec<String>> = if editing {
        config
            .tags
            .iter()
            .filter(|(_, spec)| spec.kind == TagKind::Asset)
            .filter_map(|(name, spec)| Some((name, asset_values(project, spec.path.as_deref()?))))
            .collect()
    } else {
        BTreeMap::new()
    };
    let script = lower(project);
    let problems = check(project, &script);
    let stats = stats(&script);

    let mut paths = image_paths(project, &script);
    for (name, values) in &available {
        let spec = &config.tags[*name];
        if spec.show.as_deref() == Some("face")
            && let Some(pattern) = &spec.path
        {
            paths.extend(values.iter().map(|v| pattern.replace("{value}", v)));
        }
    }
    let mut images = BTreeMap::new();
    for path in paths {
        if let Some(uri) = thumbnail(&project.resolve(&path)) {
            images.insert(path, uri);
        }
    }

    ReadView {
        title: config
            .name
            .clone()
            .unwrap_or_else(|| folder_name(&project.root)),
        places: config
            .places
            .iter()
            .map(|p| PlaceView {
                id: p.id.clone(),
                name: p.name.clone(),
                group: p.group.clone(),
                files: p.files.clone(),
                labels: p.labels.clone(),
                exclude_labels: p.exclude_labels.clone(),
            })
            .collect(),
        characters: config
            .characters
            .iter()
            .map(|c| CharacterView {
                id: c.id.clone(),
                name: c.name.clone(),
                speakers: c.speakers.clone(),
                portrait: c.portrait.clone().filter(|p| images.contains_key(p)),
                role: c.role.clone(),
            })
            .collect(),
        tags: config
            .tags
            .iter()
            .map(|(name, spec)| TagView {
                name: name.clone(),
                kind: match spec.kind {
                    TagKind::Flag => TagKindView::Flag,
                    TagKind::Enum => TagKindView::Enum,
                    TagKind::Asset => TagKindView::Asset,
                    TagKind::Text => TagKindView::Text,
                },
                values: spec.values.clone(),
                path: spec.path.clone(),
                show: spec.show.clone(),
                description: spec.description.clone(),
                available: available.get(name).cloned().unwrap_or_default(),
            })
            .collect(),
        variants: config
            .variants
            .iter()
            .map(|v| VariantView {
                name: v.name.clone(),
                condition: v.condition.clone(),
                if_true: v.if_true.clone(),
                if_false: v.if_false.clone(),
            })
            .collect(),
        flags: flags(project, &script),
        flag_functions: FlagFunctions {
            set: config.flags.set.clone(),
            read: config.flags.read.clone(),
            match_subjects: config.flags.match_subjects.clone(),
        },
        documents: config
            .documents
            .iter()
            .filter_map(|d| {
                let text = std::fs::read_to_string(project.resolve_document(&d.path)).ok()?;
                let title = d.title.clone().unwrap_or_else(|| d.path.clone());
                Some(DocumentView {
                    title,
                    path: d.path.clone(),
                    text,
                })
            })
            .collect(),
        images,
        stats,
        problems,
        script,
    }
}

fn folder_name(root: &Path) -> String {
    root.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Script".to_owned())
}

/// Every flag the script sets or reads, and every flag the project file gives a meaning.
fn flags(project: &Project, script: &Script) -> Vec<FlagView> {
    let meanings = &project.config.flags.meanings;
    let mut by_flag: BTreeMap<&str, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for flag in meanings.keys() {
        by_flag.entry(flag).or_default();
    }
    for reference in &script.flags {
        let entry = by_flag.entry(&reference.flag).or_default();
        let list = match reference.access {
            FlagAccess::Set => &mut entry.0,
            FlagAccess::Read => &mut entry.1,
        };
        if let Some(label) = &reference.label
            && !list.contains(label)
        {
            list.push(label.clone());
        }
    }
    by_flag
        .into_iter()
        .map(|(flag, (set_by, read_by))| FlagView {
            flag: flag.to_owned(),
            meaning: meanings.get(flag).cloned(),
            set_by,
            read_by,
        })
        .collect()
}

/// Portraits, and the files `face` tags point at.
fn image_paths(project: &Project, script: &Script) -> BTreeSet<String> {
    let mut paths: BTreeSet<String> = project
        .config
        .characters
        .iter()
        .filter_map(|c| c.portrait.clone())
        .collect();
    let faces: Vec<(&String, &str)> = project
        .config
        .tags
        .iter()
        .filter(|(_, spec)| spec.kind == TagKind::Asset && spec.show.as_deref() == Some("face"))
        .filter_map(|(name, spec)| Some((name, spec.path.as_deref()?)))
        .collect();
    if faces.is_empty() {
        return paths;
    }
    let mut visit = |tags: &[crate::model::Tag]| {
        for tag in tags {
            if let (Some((_, pattern)), Some(value)) = (
                faces.iter().find(|(name, _)| **name == tag.name),
                &tag.value,
            ) {
                paths.insert(pattern.replace("{value}", value));
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
            crate::model::walk(body, &mut |node| match &node.kind {
                crate::model::NodeKind::Line(line) => visit(&line.tags),
                crate::model::NodeKind::Menu { choices } => {
                    choices.iter().for_each(|c| visit(&c.tags))
                }
                _ => {}
            });
        }
    }
    paths
}

/// The values of an asset tag whose files exist: for `faces/{value}.png`, the names of the
/// `.png` files in `faces/`. Only patterns with `{value}` in the file name are listed.
fn asset_values(project: &Project, pattern: &str) -> Vec<String> {
    let Some((before, after)) = pattern.split_once("{value}") else {
        return Vec::new();
    };
    if after.contains('/') {
        return Vec::new();
    }
    let (dir, prefix) = before.rsplit_once('/').unwrap_or(("", before));
    let Ok(entries) = std::fs::read_dir(project.resolve(dir)) else {
        return Vec::new();
    };
    let mut values: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|name| Some(name.strip_prefix(prefix)?.strip_suffix(after)?.to_owned()))
        .filter(|v| !v.is_empty())
        .collect();
    values.sort();
    values
}

/// A small JPEG of an image, on white, as a data URI. None when it does not read.
pub fn thumbnail(path: &Path) -> Option<String> {
    let image = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let small = image.thumbnail(THUMBNAIL, THUMBNAIL).to_rgba8();
    let mut flat = image::RgbImage::new(small.width(), small.height());
    for (x, y, pixel) in small.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let over_white =
            |c: u8| ((u16::from(c) * u16::from(a) + 255 * (255 - u16::from(a))) / 255) as u8;
        flat.put_pixel(
            x,
            y,
            image::Rgb([over_white(r), over_white(g), over_white(b)]),
        );
    }
    let mut bytes = Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85)
        .encode_image(&flat)
        .ok()?;
    let data = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
    Some(format!("data:image/jpeg;base64,{data}"))
}
