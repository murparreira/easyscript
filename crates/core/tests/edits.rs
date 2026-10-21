//! Each edit on a small script: what the file looks like after it, or why it was refused.

use es_core::edit::{Edit, NewLine, apply};
use expect_test::{Expect, expect};

const SCRIPT: &str = "\
~ start
Keeper: Another storm. [#face=keeper_calm, mood=calm]
A gull cries.
- Ask about the oil [if not has_flag(\"asked\")]
\tKeeper: In the shed.
# ✏ prod
- Leave => bye
if Globals.easy()
\tKeeper: Easy.
else
\tKeeper: Hard.
=> END
";

/// Byte offset where 1-based line `n` starts.
fn line(text: &str, n: usize) -> usize {
    text.split_inclusive('\n').take(n - 1).map(str::len).sum()
}

/// Apply an edit about line `n` of `text`; show the file after it, or the refusal.
fn check(text: &str, n: usize, make: impl Fn(usize) -> Edit, expected: Expect) {
    let shown = match apply(text, &make(line(text, n))) {
        Ok(applied) => {
            assert_eq!(
                es_syntax::parse(&applied.text).syntax().to_string(),
                applied.text
            );
            // Carriage returns shown, so CRLF files can be checked in an LF source file.
            let shown = applied.text.replace('\r', "\\r");
            format!(
                "at line {}\n{shown}",
                applied.text[..applied.at].matches('\n').count() + 1
            )
        }
        Err(e) => format!("refused: {e}"),
    };
    expected.assert_eq(&shown);
}

fn text(at: usize, text: &str) -> Edit {
    Edit::SetText {
        at,
        text: text.into(),
    }
}

#[test]
fn set_text_keeps_tags() {
    check(
        SCRIPT,
        2,
        |at| text(at, "A calm night."),
        expect![[r#"
        at line 2
        ~ start
        Keeper: A calm night. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
}

#[test]
fn set_text_escapes() {
    check(
        SCRIPT,
        3,
        |at| text(at, "Note: it is late.\nVery late. [#not a tag]"),
        expect![[r#"
        at line 3
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        Note\: it is late.\nVery late. \[#not a tag]
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        3,
        |at| text(at, "if only it would stop"),
        expect![[r#"
        at line 3
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        \if only it would stop
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        |at| text(at, "go => there"),
        expect!["refused: `=>` in the words would read as a jump"],
    );
    check(
        SCRIPT,
        2,
        |at| text(at, "   "),
        expect!["refused: a line needs some words"],
    );
    check(
        SCRIPT,
        8,
        |at| text(at, "words"),
        expect!["refused: only a line of dialogue or a choice has text, a speaker and tags"],
    );
}

#[test]
fn set_speaker() {
    check(
        SCRIPT,
        3,
        |at| Edit::SetSpeaker {
            at,
            speaker: Some("Sailor".into()),
        },
        expect![[r#"
        at line 3
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        Sailor: A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        |at| Edit::SetSpeaker { at, speaker: None },
        expect![[r#"
        at line 2
        ~ start
        Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        |at| Edit::SetSpeaker {
            at,
            speaker: Some("Keeper".into()),
        },
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Keeper: Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        |at| Edit::SetSpeaker {
            at,
            speaker: Some("Old: Man".into()),
        },
        expect!["refused: a speaker's name cannot have `:`, `[`, `]`, `#` or a line break"],
    );
}

#[test]
fn set_and_remove_tags() {
    let set = |name: &'static str, value: Option<&'static str>| {
        move |at| Edit::SetTag {
            at,
            name: name.into(),
            value: value.map(Into::into),
        }
    };
    check(
        SCRIPT,
        2,
        set("mood", Some("angry")),
        expect![[r#"
        at line 2
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=angry]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        set("thought", None),
        expect![[r#"
        at line 2
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm, thought]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        3,
        set("face", Some("gull_1")),
        expect![[r#"
        at line 3
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries. [#face=gull_1]
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        set("thought", None),
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [#thought] [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    let remove = |name: &'static str| {
        move |at| Edit::RemoveTag {
            at,
            name: name.into(),
        }
    };
    check(
        SCRIPT,
        2,
        remove("face"),
        expect![[r#"
        at line 2
        ~ start
        Keeper: Another storm. [#mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        remove("mood"),
        expect![[r#"
        at line 2
        ~ start
        Keeper: Another storm. [#face=keeper_calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    let both = apply(
        SCRIPT,
        &Edit::RemoveTag {
            at: line(SCRIPT, 2),
            name: "face".into(),
        },
    )
    .unwrap()
    .text;
    check(
        &both,
        2,
        remove("mood"),
        expect![[r#"
        at line 2
        ~ start
        Keeper: Another storm.
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
}

#[test]
fn conditions() {
    let set = |c: Option<&'static str>| {
        move |at| Edit::SetCondition {
            at,
            condition: c.map(Into::into),
        }
    };
    check(
        SCRIPT,
        8,
        set(Some("Globals.hard()")),
        expect![[r#"
        at line 8
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.hard()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        set(None),
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        7,
        set(Some("has_flag(\"oil\")")),
        expect![[r#"
        at line 7
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye [if has_flag("oil")]
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        10,
        set(Some("x")),
        expect!["refused: an `else` has no condition"],
    );
    check(
        SCRIPT,
        8,
        set(Some("a $ b")),
        expect!["refused: `a $ b` does not read as a condition"],
    );
}

#[test]
fn insert_lines() {
    let new = |choice: bool, speaker: Option<&'static str>, text: &'static str| NewLine {
        choice,
        speaker: speaker.map(Into::into),
        text: text.into(),
        ..Default::default()
    };
    check(
        SCRIPT,
        2,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: new(false, Some("Sailor"), "Will it hold?"),
        },
        expect![[r#"
        at line 3
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        Sailor: Will it hold?
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: new(true, None, "Say nothing"),
        },
        expect![[r#"
        at line 6
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        - Say nothing
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        7,
        |at| Edit::InsertLine {
            at,
            after: false,
            under: false,
            line: new(true, None, "Wait"),
        },
        expect![[r#"
        at line 6
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        - Wait
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        5,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: new(false, None, "- not a choice"),
        },
        expect![[r#"
        at line 6
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        	\- not a choice
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
}

#[test]
fn insert_a_choice_with_tags_and_condition() {
    let line = NewLine {
        choice: true,
        speaker: None,
        text: "Light the lamp".into(),
        tags: vec![es_core::edit::NewTag {
            name: "thought".into(),
            value: None,
        }],
        condition: Some("has_flag(\"oil\")".into()),
        jump: None,
    };
    check(
        SCRIPT,
        7,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: line.clone(),
        },
        expect![[r#"
        at line 8
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        - Light the lamp [#thought] [if has_flag("oil")]
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
}

#[test]
fn remove_lines() {
    check(
        SCRIPT,
        7,
        |at| Edit::RemoveLine { at },
        expect![[r#"
        at line 6
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        |at| Edit::RemoveLine { at },
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        9,
        |at| Edit::RemoveLine { at },
        expect!["refused: this is the only line under its condition; remove the condition instead"],
    );
}

#[test]
fn move_lines() {
    check(
        SCRIPT,
        7,
        |at| Edit::MoveLine { at, up: true },
        expect![[r#"
        at line 5
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        # ✏ prod
        - Leave => bye
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        |at| Edit::MoveLine { at, up: true },
        expect!["refused: lines do not move across a label's title"],
    );
    check(
        SCRIPT,
        2,
        |at| Edit::MoveLine { at, up: false },
        expect![[r#"
        at line 3
        ~ start
        A gull cries.
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
}

#[test]
fn crlf_and_no_final_newline() {
    let crlf = "~ a\r\nKeeper: One.\r\nKeeper: Two.";
    check(
        crlf,
        2,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: NewLine {
                choice: false,
                speaker: None,
                text: "Middle.".into(),
                ..Default::default()
            },
        },
        expect![[r#"
        at line 3
        ~ a\r
        Keeper: One.\r
        Middle.\r
        Keeper: Two."#]],
    );
    check(
        crlf,
        3,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: NewLine {
                choice: false,
                speaker: None,
                text: "Last.".into(),
                ..Default::default()
            },
        },
        expect![[r#"
        at line 4
        ~ a\r
        Keeper: One.\r
        Keeper: Two.\r
        Last."#]],
    );
    check(
        crlf,
        3,
        |at| Edit::MoveLine { at, up: true },
        expect![[r#"
        at line 2
        ~ a\r
        Keeper: Two.\r
        Keeper: One."#]],
    );
}

mod random {
    use super::*;
    use proptest::prelude::*;

    const PIECES: &[&str] = &[
        "\n",
        "\t",
        "    ",
        "~ a\n",
        "Keeper: ",
        "Hi",
        " there",
        "- ",
        "Choice",
        " [#face=x]",
        " [#a, b=c]",
        " [if x]",
        " => a",
        "if x\n",
        "elif y\n",
        "else\n",
        "# note\n",
        "%",
        "| ",
        ": ",
        "\\",
        "[",
        "]",
    ];
    const WORDS: &[&str] = &[
        "Hello.",
        "a: b",
        "if x",
        "- dash",
        "[#t]",
        "x => y",
        "",
        "two\nlines",
        "ç ✓",
    ];

    fn script() -> impl Strategy<Value = String> {
        prop::collection::vec(prop::sample::select(PIECES), 1..60).prop_map(|p| p.concat())
    }

    fn edit(at: usize) -> impl Strategy<Value = Edit> {
        let word = prop::sample::select(WORDS).prop_map(str::to_owned);
        let name =
            prop::sample::select(&["face", "mood", "thought", "a"][..]).prop_map(str::to_owned);
        prop_oneof![
            word.clone()
                .prop_map(move |text| Edit::SetText { at, text }),
            prop::option::of(prop::sample::select(&["Keeper", "Sailor", "Old Man"][..])).prop_map(
                move |s| Edit::SetSpeaker {
                    at,
                    speaker: s.map(Into::into)
                }
            ),
            (name.clone(), prop::option::of(word.clone()))
                .prop_map(move |(name, value)| Edit::SetTag { at, name, value }),
            name.prop_map(move |name| Edit::RemoveTag { at, name }),
            prop::option::of(prop::sample::select(
                &["x", "has(\"y\")", "a and not b"][..]
            ))
            .prop_map(move |c| Edit::SetCondition {
                at,
                condition: c.map(Into::into)
            }),
            (any::<bool>(), any::<bool>(), word).prop_map(move |(after, choice, text)| {
                Edit::InsertLine {
                    at,
                    after,
                    under: false,
                    line: NewLine {
                        choice,
                        speaker: None,
                        text,
                        ..Default::default()
                    },
                }
            }),
            Just(Edit::RemoveLine { at }),
            any::<bool>().prop_map(move |up| Edit::MoveLine { at, up }),
        ]
    }

    fn line_starts(text: &str) -> Vec<(usize, usize)> {
        es_syntax::parse(text)
            .syntax()
            .descendants()
            .filter(|n| n.kind().is_line())
            .map(|n| (n.text_range().start().into(), n.text_range().end().into()))
            .collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(3000))]

        #[test]
        fn edits_never_panic_and_stay_on_their_line(
            (text, (start, end), edit) in script()
                .prop_filter("has lines", |t| !line_starts(t).is_empty())
                .prop_flat_map(|t| {
                    let lines = line_starts(&t);
                    (Just(t), prop::sample::select(lines))
                })
                .prop_flat_map(|(t, line)| (Just(t), Just(line), edit(line.0)))
        ) {
            let Ok(applied) = apply(&text, &edit) else { return Ok(()) };
            prop_assert_eq!(es_syntax::parse(&applied.text).syntax().to_string(), applied.text.clone());
            let in_line = matches!(edit, Edit::SetText { .. } | Edit::SetSpeaker { .. } | Edit::SetTag { .. } | Edit::RemoveTag { .. } | Edit::SetCondition { .. });
            if in_line {
                // Before the line and after it (and the lines nested under it), nothing changes.
                prop_assert_eq!(&applied.text[..start], &text[..start]);
                let tail = &text[end..];
                prop_assert!(applied.text.ends_with(tail), "changed after the line: {:?} -> {:?}", text, applied.text);
            }
        }
    }
}

#[test]
fn insert_under_a_choice_or_condition() {
    let new = |choice: bool, speaker: Option<&'static str>, text: &'static str| NewLine {
        choice,
        speaker: speaker.map(Into::into),
        text: text.into(),
        ..Default::default()
    };
    // After the lines there.
    check(
        SCRIPT,
        4,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: true,
            line: new(false, Some("Keeper"), "Mind the step."),
        },
        expect![[r#"
            at line 6
            ~ start
            Keeper: Another storm. [#face=keeper_calm, mood=calm]
            A gull cries.
            - Ask about the oil [if not has_flag("asked")]
            	Keeper: In the shed.
            	Keeper: Mind the step.
            # ✏ prod
            - Leave => bye
            if Globals.easy()
            	Keeper: Easy.
            else
            	Keeper: Hard.
            => END
        "#]],
    );
    // A choice with nothing under it yet: one tab in, as the file nests.
    check(
        SCRIPT,
        7,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: true,
            line: new(false, None, "The door shuts."),
        },
        expect![[r#"
            at line 8
            ~ start
            Keeper: Another storm. [#face=keeper_calm, mood=calm]
            A gull cries.
            - Ask about the oil [if not has_flag("asked")]
            	Keeper: In the shed.
            # ✏ prod
            - Leave => bye
            	The door shuts.
            if Globals.easy()
            	Keeper: Easy.
            else
            	Keeper: Hard.
            => END
        "#]],
    );
    // A file that nests with spaces.
    check(
        "~ a\n- Go\n    Keeper: Fine.\n- Stay\n",
        4,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: true,
            line: new(true, None, "Really stay"),
        },
        expect![[r#"
            at line 5
            ~ a
            - Go
                Keeper: Fine.
            - Stay
                - Really stay
        "#]],
    );
    check(
        SCRIPT,
        2,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: true,
            line: new(false, None, "No."),
        },
        expect!["refused: lines go under a choice or a condition only"],
    );
}

#[test]
fn jumps() {
    let jump = |target: Option<&'static str>| {
        move |at| Edit::SetJump {
            at,
            target: target.map(Into::into),
        }
    };
    check(
        SCRIPT,
        3,
        jump(Some("harbour")),
        expect![[r#"
        at line 3
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries. => harbour
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        2,
        jump(Some("END")),
        expect![[r#"
        at line 2
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm] => END
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        jump(Some("shed")),
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil => shed [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        7,
        jump(Some("END!")),
        expect![[r#"
        at line 7
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => END!
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        7,
        jump(None),
        expect![[r#"
        at line 7
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        12,
        jump(Some("start")),
        expect![[r#"
        at line 12
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => start
    "#]],
    );
    check(
        SCRIPT,
        12,
        jump(None),
        expect!["refused: this line only jumps; remove the line instead"],
    );
    check(
        SCRIPT,
        3,
        jump(Some("not a title")),
        expect!["refused: `not a title` is not a title to go to"],
    );
    // A line that only jumps, and a choice that goes somewhere.
    check(
        SCRIPT,
        3,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: NewLine {
                jump: Some("harbour".into()),
                ..Default::default()
            },
        },
        expect![[r#"
            at line 4
            ~ start
            Keeper: Another storm. [#face=keeper_calm, mood=calm]
            A gull cries.
            => harbour
            - Ask about the oil [if not has_flag("asked")]
            	Keeper: In the shed.
            # ✏ prod
            - Leave => bye
            if Globals.easy()
            	Keeper: Easy.
            else
            	Keeper: Hard.
            => END
        "#]],
    );
    check(
        SCRIPT,
        7,
        |at| Edit::InsertLine {
            at,
            after: true,
            under: false,
            line: NewLine {
                choice: true,
                text: "Run".into(),
                tags: vec![es_core::edit::NewTag {
                    name: "thought".into(),
                    value: None,
                }],
                condition: Some("brave".into()),
                jump: Some("harbour".into()),
                ..Default::default()
            },
        },
        expect![[r#"
            at line 8
            ~ start
            Keeper: Another storm. [#face=keeper_calm, mood=calm]
            A gull cries.
            - Ask about the oil [if not has_flag("asked")]
            	Keeper: In the shed.
            # ✏ prod
            - Leave => bye
            - Run [#thought] => harbour [if brave]
            if Globals.easy()
            	Keeper: Easy.
            else
            	Keeper: Hard.
            => END
        "#]],
    );
}

const LABELS: &str = "\
# The harbour
~ start
Keeper: Another storm.
=> END

## Only at night.
~ night
A gull cries.
# for the next one

~ last
=> END
";

#[test]
fn add_labels() {
    let add = |id: &'static str, after: bool| {
        move |at| Edit::AddLabel {
            at: after.then_some(at),
            id: id.into(),
        }
    };
    check(
        LABELS,
        2,
        add("dawn", true),
        expect![[r#"
        at line 6
        # The harbour
        ~ start
        Keeper: Another storm.
        => END

        ~ dawn
        => END

        ## Only at night.
        ~ night
        A gull cries.
        # for the next one

        ~ last
        => END
    "#]],
    );
    check(
        LABELS,
        11,
        add("dawn", true),
        expect![[r#"
        at line 14
        # The harbour
        ~ start
        Keeper: Another storm.
        => END

        ## Only at night.
        ~ night
        A gull cries.
        # for the next one

        ~ last
        => END

        ~ dawn
        => END
    "#]],
    );
    check(
        LABELS,
        1,
        add("dawn", false),
        expect![[r#"
        at line 14
        # The harbour
        ~ start
        Keeper: Another storm.
        => END

        ## Only at night.
        ~ night
        A gull cries.
        # for the next one

        ~ last
        => END

        ~ dawn
        => END
    "#]],
    );
    check(
        "~ a\nA.",
        1,
        add("b", false),
        expect![[r#"
        at line 4
        ~ a
        A.

        ~ b
        => END
    "#]],
    );
    check(
        "",
        1,
        add("first", false),
        expect![[r#"
        at line 1
        ~ first
        => END
    "#]],
    );
    check(
        LABELS,
        2,
        add("night", true),
        expect!["refused: there is a label `night` in this file already"],
    );
    check(
        LABELS,
        2,
        add("2nd", true),
        expect![
            "refused: `2nd` cannot be a title: letters, digits and `_`, not starting with a digit"
        ],
    );
    check(
        LABELS,
        3,
        add("dawn", true),
        expect!["refused: new labels go after a label's title"],
    );
}

#[test]
fn remove_labels() {
    let remove = |from: Option<usize>| move |at| Edit::RemoveLabel { at, from };
    check(
        LABELS,
        2,
        remove(None),
        expect![[r#"
        at line 2
        # The harbour
        ## Only at night.
        ~ night
        A gull cries.
        # for the next one

        ~ last
        => END
    "#]],
    );
    // With its notes above the title.
    check(
        LABELS,
        7,
        remove(Some(line(LABELS, 6))),
        expect![[r#"
        at line 6
        # The harbour
        ~ start
        Keeper: Another storm.
        => END

        # for the next one

        ~ last
        => END
    "#]],
    );
    check(
        LABELS,
        11,
        remove(None),
        expect![[r#"
        at line 10
        # The harbour
        ~ start
        Keeper: Another storm.
        => END

        ## Only at night.
        ~ night
        A gull cries.
        # for the next one
    "#]],
    );
    check(
        LABELS,
        7,
        remove(Some(line(LABELS, 3))),
        expect!["refused: the label's notes are not right above its title"],
    );
    check(
        LABELS,
        3,
        remove(None),
        expect!["refused: no label's title starts there"],
    );
}

#[test]
fn insert_code_and_conditions() {
    let code = |code: &'static str, after: bool, under: bool| {
        move |at| Edit::InsertCode {
            at,
            after,
            under,
            code: code.into(),
        }
    };
    check(
        SCRIPT,
        3,
        code("do Globals.set_flag(\"gull\")", true, false),
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        do Globals.set_flag("gull")
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        4,
        code("set gold += 1", true, true),
        expect![[r#"
        at line 6
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        	set gold += 1
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        3,
        code("Globals.set_flag(\"gull\")", true, false),
        expect!["refused: game code here starts with `do` or `set`"],
    );
    check(
        SCRIPT,
        3,
        code("do ", true, false),
        expect!["refused: game code here starts with `do` or `set`"],
    );

    let condition = |keyword: &'static str, condition: Option<&'static str>, after: bool| {
        move |at| Edit::InsertCondition {
            at,
            after,
            under: false,
            keyword: keyword.into(),
            condition: condition.map(Into::into),
            line: NewLine {
                speaker: Some("Keeper".into()),
                text: "Watch out.".into(),
                ..Default::default()
            },
        }
    };
    check(
        SCRIPT,
        3,
        condition("if", Some("has_flag(\"oil\")"), true),
        expect![[r#"
        at line 4
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        if has_flag("oil")
        	Keeper: Watch out.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    // After the last arm of an `if`: another arm.
    check(
        SCRIPT,
        10,
        condition("elif", Some("Globals.hard()"), false),
        expect![[r#"
        at line 10
        ~ start
        Keeper: Another storm. [#face=keeper_calm, mood=calm]
        A gull cries.
        - Ask about the oil [if not has_flag("asked")]
        	Keeper: In the shed.
        # ✏ prod
        - Leave => bye
        if Globals.easy()
        	Keeper: Easy.
        elif Globals.hard()
        	Keeper: Watch out.
        else
        	Keeper: Hard.
        => END
    "#]],
    );
    check(
        SCRIPT,
        3,
        condition("else", Some("x"), true),
        expect!["refused: an `else` has no condition"],
    );
    check(
        SCRIPT,
        3,
        condition("if", None, true),
        expect!["refused: a condition is one line, and not empty"],
    );
    check(
        SCRIPT,
        3,
        condition("else", None, true),
        expect!["refused: `else` does not read as a condition here"],
    );
    check(
        SCRIPT,
        12,
        condition("else", None, false),
        expect!["refused: `else` does not read as a condition here"],
    );
}
