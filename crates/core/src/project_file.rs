//! Edits to the project file that keep it as its team wrote it: comments, order, blank
//! lines. Each edit finds the table it is about and changes only that; the result must
//! still read as a project file.

use serde::{Deserialize, Serialize};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value};
use ts_rs::TS;

use crate::config::{Config, TagKind};
use crate::edit::EditError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
#[ts(export)]
pub enum ConfigEdit {
    /// Add a character, or change the one with this id.
    SetCharacter {
        id: String,
        name: String,
        speakers: Vec<String>,
        portrait: Option<String>,
        role: Option<String>,
    },
    RemoveCharacter {
        id: String,
    },
    MoveCharacter {
        id: String,
        up: bool,
    },
    /// Add a tag to the schema, or change it.
    SetTag {
        name: String,
        kind: TagKind,
        values: Vec<String>,
        path: Option<String>,
        show: Option<String>,
        description: Option<String>,
    },
    RemoveTag {
        name: String,
    },
    /// What a flag means; none removes the meaning.
    SetFlagMeaning {
        flag: String,
        meaning: Option<String>,
    },
    /// Add a place, or change the one with this id.
    SetPlace {
        id: String,
        name: String,
        group: Option<String>,
        files: Vec<String>,
        labels: Vec<String>,
        exclude_labels: Vec<String>,
    },
    RemovePlace {
        id: String,
    },
    MovePlace {
        id: String,
        up: bool,
    },
    /// Add a variant, or change the one with this name.
    SetVariant {
        name: String,
        condition: String,
        if_true: String,
        if_false: String,
    },
    RemoveVariant {
        name: String,
    },
}

fn fail<T>(message: impl Into<String>) -> Result<T, EditError> {
    Err(EditError(message.into()))
}

/// Make an edit to a project file's text (empty for a project that has none yet).
pub fn apply(text: &str, edit: &ConfigEdit) -> Result<String, EditError> {
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| EditError(format!("the project file does not read: {e}")))?;
    match edit {
        ConfigEdit::SetCharacter {
            id,
            name,
            speakers,
            portrait,
            role,
        } => {
            let id = required("A character's id", id)?;
            let table = entry(&mut doc, "characters", "id", id);
            set(table, "name", Some(name.trim().into()));
            set(table, "speakers", Some(strings(speakers)));
            set(table, "portrait", nonempty(portrait).map(Into::into));
            set(table, "role", nonempty(role).map(Into::into));
        }
        ConfigEdit::RemoveCharacter { id } => remove(&mut doc, "characters", "id", id)?,
        ConfigEdit::MoveCharacter { id, up } => move_entry(&mut doc, "characters", "id", id, *up)?,
        ConfigEdit::SetTag {
            name,
            kind,
            values,
            path,
            show,
            description,
        } => {
            let name = required("A tag's name", name)?;
            let tags = table(doc.as_table_mut(), "tags");
            let tag = table(tags, name);
            set(tag, "kind", Some(kind_name(*kind).into()));
            set(
                tag,
                "values",
                (*kind == TagKind::Enum).then(|| strings(values)),
            );
            set(
                tag,
                "path",
                nonempty(path)
                    .filter(|_| *kind == TagKind::Asset)
                    .map(Into::into),
            );
            set(tag, "show", nonempty(show).map(Into::into));
            set(tag, "description", nonempty(description).map(Into::into));
        }
        ConfigEdit::RemoveTag { name } => {
            let removed = doc
                .get_mut("tags")
                .and_then(Item::as_table_like_mut)
                .and_then(|t| t.remove(name));
            if removed.is_none() {
                return fail(format!("no tag `{name}` in the project file"));
            }
        }
        ConfigEdit::SetFlagMeaning { flag, meaning } => {
            let flag = required("A flag", flag)?;
            let flags = table(doc.as_table_mut(), "flags");
            let meanings = flags.entry("meanings").or_insert_with(|| {
                let mut t = Table::new();
                t.set_implicit(false);
                Item::Table(t)
            });
            let inline = meanings.is_inline_table();
            let Some(table) = meanings.as_table_like_mut() else {
                return fail("`flags.meanings` is not a table");
            };
            match nonempty(meaning) {
                Some(meaning) => set_like(table, flag, meaning.into()),
                None => {
                    table.remove(flag);
                }
            }
            if inline && let Some(inline) = meanings.as_inline_table_mut() {
                inline.fmt();
            }
        }
        ConfigEdit::SetPlace {
            id,
            name,
            group,
            files,
            labels,
            exclude_labels,
        } => {
            let id = required("A place's id", id)?;
            let table = entry(&mut doc, "places", "id", id);
            set(table, "name", Some(name.trim().into()));
            set(table, "group", nonempty(group).map(Into::into));
            set(table, "files", Some(strings(files)));
            set(
                table,
                "labels",
                (!labels.is_empty()).then(|| strings(labels)),
            );
            set(
                table,
                "exclude_labels",
                (!exclude_labels.is_empty()).then(|| strings(exclude_labels)),
            );
        }
        ConfigEdit::RemovePlace { id } => remove(&mut doc, "places", "id", id)?,
        ConfigEdit::MovePlace { id, up } => move_entry(&mut doc, "places", "id", id, *up)?,
        ConfigEdit::SetVariant {
            name,
            condition,
            if_true,
            if_false,
        } => {
            let name = required("A variant's name", name)?;
            let table = entry(&mut doc, "variants", "name", name);
            set(table, "condition", Some(condition.trim().into()));
            set(table, "true", Some(if_true.trim().into()));
            set(table, "false", Some(if_false.trim().into()));
        }
        ConfigEdit::RemoveVariant { name } => remove(&mut doc, "variants", "name", name)?,
    }
    let out = doc.to_string();
    let config = Config::parse(&out).map_err(|e| {
        EditError(format!(
            "the edit would leave the project file unreadable: {e}"
        ))
    })?;
    config.compile().map_err(|e| EditError(e.to_string()))?;
    Ok(out)
}

fn required<'a>(what: &str, value: &'a str) -> Result<&'a str, EditError> {
    let value = value.trim();
    if value.is_empty() {
        fail(format!("{what} cannot be empty"))
    } else {
        Ok(value)
    }
}

fn nonempty(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

fn strings(values: &[String]) -> Value {
    values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect::<toml_edit::Array>()
        .into()
}

fn kind_name(kind: TagKind) -> &'static str {
    match kind {
        TagKind::Flag => "flag",
        TagKind::Enum => "enum",
        TagKind::Asset => "asset",
        TagKind::Text => "text",
    }
}

/// Set a key, keeping the comments around its old value; none removes it.
fn set(table: &mut Table, key: &str, value: Option<Value>) {
    match value {
        Some(value) => set_like(table, key, value),
        None => {
            table.remove(key);
        }
    }
}

fn set_like(table: &mut dyn toml_edit::TableLike, key: &str, mut value: Value) {
    match table.get_mut(key) {
        Some(Item::Value(old)) => {
            let decor = old.decor().clone();
            *value.decor_mut() = decor;
            *old = value;
        }
        _ => {
            table.insert(key, Item::Value(value));
        }
    }
}

/// A table under `parent`, made when missing.
fn table<'a>(parent: &'a mut Table, key: &str) -> &'a mut Table {
    let item = parent.entry(key).or_insert_with(|| {
        // Only its own keys or tables give it a header.
        let mut t = Table::new();
        t.set_implicit(true);
        Item::Table(t)
    });
    if !item.is_table() {
        *item = Item::Table(Table::new());
    }
    item.as_table_mut().expect("made a table")
}

fn array<'a>(doc: &'a mut DocumentMut, key: &str) -> &'a mut ArrayOfTables {
    let item = doc
        .as_table_mut()
        .entry(key)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    if !item.is_array_of_tables() {
        *item = Item::ArrayOfTables(ArrayOfTables::new());
    }
    item.as_array_of_tables_mut()
        .expect("made an array of tables")
}

fn index_of(entries: &ArrayOfTables, field: &str, value: &str) -> Option<usize> {
    entries
        .iter()
        .position(|t| t.get(field).and_then(Item::as_str) == Some(value))
}

/// The entry of `[[key]]` whose `field` is `value`, added after the others when missing.
fn entry<'a>(doc: &'a mut DocumentMut, key: &str, field: &str, value: &str) -> &'a mut Table {
    let entries = array(doc, key);
    let index = match index_of(entries, field, value) {
        Some(index) => index,
        None => {
            let last = entries.iter().filter_map(Table::position).max();
            let mut table = Table::new();
            table.insert(field, toml_edit::value(value));
            // Right after the last entry, not at the end of the file.
            table.set_position(last);
            entries.push(table);
            entries.len() - 1
        }
    };
    entries.get_mut(index).expect("in range")
}

fn remove(doc: &mut DocumentMut, key: &str, field: &str, value: &str) -> Result<(), EditError> {
    let entries = array(doc, key);
    let Some(index) = index_of(entries, field, value) else {
        return fail(format!("no {key} entry with {field} `{value}`"));
    };
    entries.remove(index);
    if entries.is_empty() {
        doc.as_table_mut().remove(key);
    }
    Ok(())
}

/// Swap an entry with its neighbour, and their places in the file.
fn move_entry(
    doc: &mut DocumentMut,
    key: &str,
    field: &str,
    value: &str,
    up: bool,
) -> Result<(), EditError> {
    let entries = array(doc, key);
    let Some(index) = index_of(entries, field, value) else {
        return fail(format!("no {key} entry with {field} `{value}`"));
    };
    let other = if up {
        index.checked_sub(1)
    } else {
        Some(index + 1).filter(|&i| i < entries.len())
    };
    let Some(other) = other else {
        return fail(if up {
            "already the first"
        } else {
            "already the last"
        });
    };
    let (low, high) = (index.min(other), index.max(other));
    let mut second = entries.remove(high);
    let mut first = entries.remove(low);
    let (p1, p2) = (first.position(), second.position());
    first.set_position(p2);
    second.set_position(p1);
    entries.insert(low, second);
    entries.insert(high, first);
    Ok(())
}
