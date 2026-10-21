//! Every export of a small project, as snapshots (`UPDATE_EXPECT=1 cargo test` rewrites
//! them after a deliberate change).

use std::path::Path;

use es_core::{Project, Script};
use es_formats::export::{Language, fountain, json, translation, voice};
use expect_test::expect_file;

fn ferry() -> (Project, Script) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ferry");
    let project = Project::load(&root, None).unwrap();
    let script = es_core::lower(&project);
    (project, script)
}

#[test]
fn json_export() {
    let (project, script) = ferry();
    let text = json::to_string(&project, &script);
    expect_file!["snapshots/ferry.json"].assert_eq(&text);
    let back: json::ScriptExport = serde_json::from_str(&text).unwrap();
    assert_eq!(back, json::export(&project, &script), "reads back");
}

#[test]
fn the_json_schema_is_committed() {
    expect_file!["../../../docs/formats/script.schema.json"].assert_eq(&json::schema());
}

#[test]
fn translation_table() {
    let (project, script) = ferry();
    let options = translation::Options {
        extra_locales: vec!["pt_BR".into()],
        ..Default::default()
    };
    let text = translation::to_csv(&project, &script, &options, None).unwrap();
    expect_file!["snapshots/ferry-translation.csv"].assert_eq(&text);
}

#[test]
fn translation_table_keeps_translations() {
    let (project, script) = ferry();
    let existing = "keys,en,pt_BR,_character,_notes\n\
        BOARD_1,Welcome!,Bem-vindo a bordo!,Captain,\n\
        Gone line,Gone line,Fala removida,Captain,\n";
    let options = translation::Options {
        extra_locales: vec!["pt_BR".into(), "es".into()],
        ..Default::default()
    };
    let text = translation::to_csv(&project, &script, &options, Some(existing)).unwrap();
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows[0], "keys,en,pt_BR,_character,_notes,es");
    assert_eq!(
        rows[1],
        "BOARD_1,Welcome aboard!,Bem-vindo a bordo!,Captain,The captain greets the player. Keep it short.,",
        "the text is the script's now; the translation stays"
    );
    assert_eq!(
        rows.last().unwrap(),
        &"Gone line,Gone line,Fala removida,Captain,,",
        "a line gone from the script keeps its row, last"
    );
    let refused = translation::to_csv(&project, &script, &options, Some("a,b\n")).unwrap_err();
    assert!(refused.0.contains("not a translation table"), "{refused}");
}

#[test]
fn voice_scripts() {
    let (project, script) = ferry();
    let en = Language::En;
    expect_file!["snapshots/ferry-voice.md"]
        .assert_eq(&voice::to_markdown(&project, &script, None, en));
    expect_file!["snapshots/ferry-voice.csv"].assert_eq(&voice::to_csv(&project, &script, en));
    expect_file!["snapshots/ferry-voice.pt-BR.md"].assert_eq(&voice::to_markdown(
        &project,
        &script,
        None,
        Language::PtBr,
    ));
    let captain = voice::to_markdown(&project, &script, Some("captain"), en);
    assert!(captain.starts_with("# The Captain: voice script"));
    assert!(!captain.contains("Deckhand: voice script"));
}

#[test]
fn screenplay() {
    let (project, script) = ferry();
    expect_file!["snapshots/ferry.fountain"].assert_eq(&fountain::to_fountain(
        &project,
        &script,
        Language::En,
    ));
    expect_file!["snapshots/ferry.pt-BR.fountain"].assert_eq(&fountain::to_fountain(
        &project,
        &script,
        Language::PtBr,
    ));
}

#[test]
fn dialogue_files_as_a_zip() {
    use std::io::Read;
    let (project, _) = ferry();
    let bytes = es_formats::export::archive::dialogue_zip(&project, &[]).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(zip.len(), project.files.len());
    for file in &project.files {
        let mut text = String::new();
        zip.by_name(&file.path)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, file.text, "{} as it is", file.path);
    }
    let none = es_formats::export::archive::dialogue_zip(&project, &["elsewhere.dialogue".into()]);
    assert!(none.is_err());
}
