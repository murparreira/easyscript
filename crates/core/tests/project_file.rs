//! Edits to a project file keep it as written: only what an edit is about changes.

use es_core::config::TagKind;
use es_core::project_file::{ConfigEdit, apply};
use expect_test::{Expect, expect};

const FILE: &str = r#"# The lighthouse's project file.
name = "Lighthouse"

[sources]
dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

# Cast, in order of appearance.
[[characters]]
id = "keeper"
name = "Keeper" # the old man
speakers = ["Keeper"]

[[characters]]
id = "sailor"
name = "Sailor"
speakers = ["Sailor"]

[tags.mood]
kind = "enum"
values = ["calm"]

[flags]
set = ["set_flag"]
meanings = { oil = "The oil was found." }
"#;

fn check(text: &str, edit: ConfigEdit, expected: Expect) {
    let shown = match apply(text, &edit) {
        Ok(out) => out,
        Err(e) => format!("refused: {e}"),
    };
    expected.assert_eq(&shown);
}

fn character(id: &str, name: &str) -> ConfigEdit {
    ConfigEdit::SetCharacter {
        id: id.into(),
        name: name.into(),
        speakers: vec![name.into()],
        portrait: None,
        role: None,
        voices: vec![],
    }
}

#[test]
fn change_a_character_keeps_comments() {
    check(
        FILE,
        ConfigEdit::SetCharacter {
            id: "keeper".into(),
            name: "The Keeper".into(),
            speakers: vec!["Keeper".into(), "Old Man".into()],
            portrait: Some("faces/keeper.png".into()),
            role: None,
            voices: vec![],
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "The Keeper" # the old man
        speakers = ["Keeper", "Old Man"]
        portrait = "faces/keeper.png"

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
}

#[test]
fn add_move_remove_characters() {
    check(
        FILE,
        character("gull", "Gull"),
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [[characters]]
        id = "gull"
        name = "Gull"
        speakers = ["Gull"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
    check(
        FILE,
        ConfigEdit::MoveCharacter {
            id: "sailor".into(),
            up: true,
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
    check(
        FILE,
        ConfigEdit::RemoveCharacter {
            id: "keeper".into(),
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
    check(
        FILE,
        ConfigEdit::RemoveCharacter {
            id: "nobody".into(),
        },
        expect!["refused: no characters entry with id `nobody`"],
    );
}

#[test]
fn tags() {
    let tag = |name: &str, kind, values: &[&str]| ConfigEdit::SetTag {
        name: name.into(),
        kind,
        values: values.iter().map(|v| v.to_string()).collect(),
        path: None,
        show: (kind == TagKind::Flag).then(|| "thought".into()),
        description: None,
    };
    check(
        FILE,
        tag("mood", TagKind::Enum, &["calm", "angry"]),
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm", "angry"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
    check(
        FILE,
        tag("thought", TagKind::Flag, &[]),
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [tags.thought]
        kind = "flag"
        show = "thought"

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
    check(
        FILE,
        ConfigEdit::RemoveTag {
            name: "mood".into(),
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }
    "##]],
    );
}

#[test]
fn flags_places_variants() {
    check(
        FILE,
        ConfigEdit::SetFlagMeaning {
            flag: "storm".into(),
            meaning: Some("A storm is coming.".into()),
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found.", storm = "A storm is coming." }
    "##]],
    );
    check(
        FILE,
        ConfigEdit::SetFlagMeaning {
            flag: "oil".into(),
            meaning: None,
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = {}
    "##]],
    );
    check(
        FILE,
        ConfigEdit::SetPlace {
            id: "pier".into(),
            name: "The Pier".into(),
            group: Some("Harbour".into()),
            files: vec!["dialogue/pier.dialogue".into()],
            labels: vec![],
            exclude_labels: vec![],
        },
        expect![[r##"
            # The lighthouse's project file.
            name = "Lighthouse"

            [sources]
            dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

            # Cast, in order of appearance.
            [[characters]]
            id = "keeper"
            name = "Keeper" # the old man
            speakers = ["Keeper"]

            [[characters]]
            id = "sailor"
            name = "Sailor"
            speakers = ["Sailor"]

            [tags.mood]
            kind = "enum"
            values = ["calm"]

            [flags]
            set = ["set_flag"]
            meanings = { oil = "The oil was found." }

            [[places]]
            id = "pier"
            name = "The Pier"
            group = "Harbour"
            files = ["dialogue/pier.dialogue"]
        "##]],
    );
    check(
        FILE,
        ConfigEdit::SetVariant {
            name: "Mode".into(),
            condition: "Globals.easy()".into(),
            if_true: "Easy".into(),
            if_false: "Hard".into(),
        },
        expect![[r##"
        # The lighthouse's project file.
        name = "Lighthouse"

        [sources]
        dialogue = ["dialogue/**/*.dialogue"]  # just the one folder

        # Cast, in order of appearance.
        [[characters]]
        id = "keeper"
        name = "Keeper" # the old man
        speakers = ["Keeper"]

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]

        [tags.mood]
        kind = "enum"
        values = ["calm"]

        [flags]
        set = ["set_flag"]
        meanings = { oil = "The oil was found." }

        [[variants]]
        name = "Mode"
        condition = "Globals.easy()"
        true = "Easy"
        false = "Hard"
    "##]],
    );
}

#[test]
fn a_project_with_no_file_yet() {
    let one = apply("", &character("keeper", "Keeper")).unwrap();
    let two = apply(
        &one,
        &ConfigEdit::SetFlagMeaning {
            flag: "oil".into(),
            meaning: Some("Found.".into()),
        },
    )
    .unwrap();
    expect![[r#"
        [[characters]]
        id = "keeper"
        name = "Keeper"
        speakers = ["Keeper"]

        [flags.meanings]
        oil = "Found."
    "#]]
    .assert_eq(&two);
}
