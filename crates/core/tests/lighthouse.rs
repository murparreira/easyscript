//! The lighthouse fixture: a small project that trips every check once, next to cases
//! that must not trip them.

use std::path::Path;

use dm_core::model::{ArmKeyword, FlagAccess, NodeKind, Segment};
use dm_core::{Project, check, lower, stats};
use expect_test::expect;

fn load() -> Project {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lighthouse");
    Project::load(&root, None).expect("fixture loads")
}

#[test]
fn checks() {
    let project = load();
    let script = lower(&project);
    let report: Vec<String> = check(&project, &script)
        .iter()
        .map(ToString::to_string)
        .collect();
    expect![[r#"
        dialogue-maker.toml:21:38: error[unknown-place-file]: place `tower`: `dialogue/missing.dialogue` is not one of the dialogue files
        dialogue-maker.toml:22:25: warning[unknown-place-label]: place `tower`: no label `no_such_label` in its files
        dialogue-maker.toml:35:13: error[missing-portrait]: sailor's portrait `faces/sailor.png` does not exist
        dialogue-maker.toml:62:9: error[missing-document]: document `docs/missing.md` cannot be read
        dialogue/latin1.dialogue:1:1: error[unreadable-file]: cannot read this file (not UTF-8); it is left alone
        dialogue/pier.dialogue:7:32: error[missing-asset]: `faces/sailor_worried.png` does not exist (1 line)
        dialogue/pier.dialogue:8:1: warning[unknown-speaker]: speaker `Stranger` is no character's (1 line)
        dialogue/pier.dialogue:9:1: warning[missing-voice]: keeper has no `sleepy` voice recorded (1 line)
        dialogue/pier.dialogue:9:15: warning[bad-tag-value]: `sleepy` is not one of mood's values (calm, angry)
        dialogue/pier.dialogue:9:28: warning[bad-tag-value]: `thought` takes no value, has `yes`
        dialogue/pier.dialogue:9:41: warning[unknown-tag]: tag `volume` is not in the project file (1 line)
        dialogue/pier.dialogue:15:1: error[unknown-jump-target]: no title `nowhere` to jump to
        dialogue/pier.dialogue:22:1: warning[unreachable-label]: nothing leads to `forgotten`: no jump, no code naming it, not an entry point
        dialogue/pier.dialogue:29:1: warning[unknown-speaker]: speaker `Gull` is no character's (1 line)
        dialogue/pier.dialogue:37:3: error[duplicate-label]: `keeper_intro` is already a title in this file
        dialogue/tower.dialogue:7:13: error[syntax]: cannot read `$`
        docs/walkthrough.md:3:44: error[broken-link]: no label `keeper_goodbye`"#]].assert_eq(&report.join("\n"));
}

#[test]
fn counts() {
    let project = load();
    let stats = stats(&lower(&project));
    expect![[r#"
        Stats {
            files: 2,
            labels: 8,
            lines: 14,
            choices: 2,
            menus: 1,
            branches: 1,
            jumps: 8,
            effects: 2,
            words: 46,
            speakers: [
                Count {
                    name: "Keeper",
                    count: 11,
                },
                Count {
                    name: "Gull",
                    count: 1,
                },
                Count {
                    name: "Sailor",
                    count: 1,
                },
                Count {
                    name: "Stranger",
                    count: 1,
                },
            ],
            tags: [
                Count {
                    name: "face",
                    count: 6,
                },
                Count {
                    name: "mood",
                    count: 2,
                },
                Count {
                    name: "thought",
                    count: 2,
                },
                Count {
                    name: "volume",
                    count: 1,
                },
            ],
            marks: [
                Count {
                    name: "production",
                    count: 1,
                },
            ],
            flags_set: 1,
            flags_read: 1,
        }
    "#]]
    .assert_debug_eq(&stats);
}

#[test]
fn model() {
    let project = load();
    let script = lower(&project);
    let pier = script
        .files
        .iter()
        .find(|f| f.path == "dialogue/pier.dialogue")
        .expect("pier");

    let titles: Vec<_> = pier.sections.iter().map(|s| s.title.as_deref()).collect();
    assert_eq!(titles, [Some("KEEPER"), Some("BIRDS")]);
    assert_eq!(pier.sections[0].notes[0].text, "Picked by keeper.gd.");

    let intro = &pier.sections[0].labels[0];
    assert_eq!(intro.id, "keeper_intro");
    assert_eq!(intro.place.as_deref(), Some("pier"));
    assert_eq!(intro.notes[0].text, "The first talk.");
    let forgotten = &pier.sections[0].labels[2];
    assert_eq!(forgotten.notes[0].text, "Nobody calls this one.");

    let NodeKind::Line(first) = &intro.body[0].kind else {
        panic!("a line first")
    };
    assert_eq!(first.speaker.as_deref(), Some("Keeper"));
    assert_eq!(
        first.text,
        [Segment::Text {
            text: "Another storm.".into()
        }]
    );
    assert_eq!(
        first
            .tags
            .iter()
            .map(|t| (t.name.as_str(), t.value.as_deref()))
            .collect::<Vec<_>>(),
        [("face", Some("keeper_calm")), ("mood", Some("calm"))]
    );

    let NodeKind::Menu { choices } = &intro.body[4].kind else {
        panic!("then a menu")
    };
    assert_eq!(choices.len(), 2);
    assert_eq!(
        choices[0].condition.as_deref(),
        Some("not has_flag(\"asked\")")
    );
    assert_eq!(choices[0].body.len(), 2);
    assert_eq!(choices[1].marks[0].who, "production");
    assert_eq!(choices[1].marks[0].note.as_deref(), Some("wording pending"));
    assert!(choices[1].jump.is_some());

    let tower = script
        .files
        .iter()
        .find(|f| f.path == "dialogue/tower.dialogue")
        .expect("tower");
    let NodeKind::Branch { arms } = &tower.sections[0].labels[0].body[1].kind else {
        panic!("a branch")
    };
    assert_eq!(
        arms.iter().map(|a| a.keyword).collect::<Vec<_>>(),
        [ArmKeyword::If, ArmKeyword::Else]
    );

    let flags: Vec<_> = script
        .flags
        .iter()
        .map(|f| (f.flag.as_str(), f.access, f.label.as_deref()))
        .collect();
    assert_eq!(
        flags,
        [
            ("asked", FlagAccess::Read, Some("keeper_intro")),
            ("asked", FlagAccess::Set, Some("keeper_intro")),
            ("asked", FlagAccess::Read, Some("tower_top")),
        ]
    );
}

#[test]
fn model_serializes() {
    let script = lower(&load());
    let json = serde_json::to_value(&script).expect("serializable");
    let first = &json["files"][0]["sections"][0]["labels"][0];
    assert_eq!(first["id"], "keeper_intro");
    assert_eq!(first["body"][0]["type"], "line");
    assert_eq!(
        first["body"][0]["text"][0],
        serde_json::json!({ "type": "text", "text": "Another storm." })
    );
}

#[test]
fn read_view() {
    let view = dm_core::read_view(&load());
    assert_eq!(view.title, "Lighthouse");
    // The keeper's portrait has a thumbnail; the sailor's file is missing.
    assert_eq!(
        view.images.keys().collect::<Vec<_>>(),
        ["faces/keeper_calm.png"]
    );
    assert!(view.images["faces/keeper_calm.png"].starts_with("data:image/jpeg;base64,"));
    let keeper = view
        .characters
        .iter()
        .find(|c| c.id == "keeper")
        .expect("keeper");
    assert_eq!(keeper.portrait.as_deref(), Some("faces/keeper_calm.png"));
    assert_eq!(
        view.characters
            .iter()
            .find(|c| c.id == "sailor")
            .and_then(|c| c.portrait.clone()),
        None
    );
    let asked = view
        .flags
        .iter()
        .find(|f| f.flag == "asked")
        .expect("asked");
    assert_eq!(asked.set_by, ["keeper_intro"]);
    assert_eq!(asked.read_by, ["keeper_intro", "tower_top"]);
    assert_eq!(view.documents.len(), 1, "the missing document is left out");
    assert_eq!(view.problems.len(), 17);
}

#[test]
fn flow() {
    let project = load();
    let script = lower(&project);
    let flow = dm_core::flow::flow(&project, &script);
    let edges: Vec<String> = flow
        .edges
        .iter()
        .map(|e| format!("{} -> {} ({:?})", e.from, e.to, e.kind))
        .collect();
    expect![[r#"
        0:keeper_intro -> 0:keeper_bye (Choice)
        0:storm_night -> 0:falls_in (FallThrough)
        0:keeper_intro -> 1:tower_top (Flag)"#]]
    .assert_eq(&edges.join("\n"));
    expect!["0:keeper_intro 0:gull_1 0:storm_night 1:tower_top"].assert_eq(&flow.entries.join(" "));
}
