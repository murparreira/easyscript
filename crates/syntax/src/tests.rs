use expect_test::{Expect, expect};

use crate::parse;

fn check(text: &str, expected: Expect) {
    let parse = parse(text);
    assert_eq!(parse.syntax().to_string(), text, "round trip");
    expected.assert_eq(&format!("{:#?}", parse.syntax()));
}

#[test]
fn title_dialogue_jump() {
    check(
        "~ start\nDetetive: Caramba! [#reaction=detetive_serio_1, voz=chateado]\n=> END\n",
        expect![[r##"
            SOURCE_FILE@0..77
              TITLE_LINE@0..8
                TILDE@0..1 "~"
                WHITESPACE@1..2 " "
                TITLE_NAME@2..7 "start"
                NEWLINE@7..8 "\n"
              DIALOGUE_LINE@8..70
                SPEAKER@8..17
                  TEXT@8..16 "Detetive"
                  COLON@16..17 ":"
                WHITESPACE@17..18 " "
                TEXT@18..26 "Caramba!"
                WHITESPACE@26..27 " "
                TAG_GROUP@27..69
                  L_BRACK@27..28 "["
                  HASH@28..29 "#"
                  TAG@29..54
                    TAG_NAME@29..37 "reaction"
                    EQ@37..38 "="
                    TAG_VALUE@38..54 "detetive_serio_1"
                  COMMA@54..55 ","
                  WHITESPACE@55..56 " "
                  TAG@56..68
                    TAG_NAME@56..59 "voz"
                    EQ@59..60 "="
                    TAG_VALUE@60..68 "chateado"
                  R_BRACK@68..69 "]"
                NEWLINE@69..70 "\n"
              GOTO_LINE@70..77
                JUMP@70..76
                  ARROW@70..72 "=>"
                  WHITESPACE@72..73 " "
                  JUMP_TARGET@73..76 "END"
                NEWLINE@76..77 "\n"
        "##]],
    );
}

#[test]
fn responses_with_bodies() {
    check(
        "- Sim [if Globals.visitou(\"rua_15\")]\n    Porteiro: Bão.\n\tPorteiro: Tá.\n- Não\n=> END",
        expect![[r#"
            SOURCE_FILE@0..86
              RESPONSE_LINE@0..73
                DASH@0..1 "-"
                WHITESPACE@1..2 " "
                TEXT@2..5 "Sim"
                WHITESPACE@5..6 " "
                INLINE_CONDITION@6..36
                  L_BRACK@6..7 "["
                  IF_KW@7..9 "if"
                  WHITESPACE@9..10 " "
                  EXPR@10..35
                    IDENT@10..17 "Globals"
                    DOT@17..18 "."
                    IDENT@18..25 "visitou"
                    L_PAREN@25..26 "("
                    STRING@26..34 "\"rua_15\""
                    R_PAREN@34..35 ")"
                  R_BRACK@35..36 "]"
                NEWLINE@36..37 "\n"
                BLOCK@37..73
                  DIALOGUE_LINE@37..57
                    INDENT@37..41 "    "
                    SPEAKER@41..50
                      TEXT@41..49 "Porteiro"
                      COLON@49..50 ":"
                    WHITESPACE@50..51 " "
                    TEXT@51..56 "Bão."
                    NEWLINE@56..57 "\n"
                  DIALOGUE_LINE@57..73
                    INDENT@57..58 "\t"
                    SPEAKER@58..67
                      TEXT@58..66 "Porteiro"
                      COLON@66..67 ":"
                    WHITESPACE@67..68 " "
                    TEXT@68..72 "Tá."
                    NEWLINE@72..73 "\n"
              RESPONSE_LINE@73..80
                DASH@73..74 "-"
                WHITESPACE@74..75 " "
                TEXT@75..79 "Não"
                NEWLINE@79..80 "\n"
              GOTO_LINE@80..86
                JUMP@80..86
                  ARROW@80..82 "=>"
                  WHITESPACE@82..83 " "
                  JUMP_TARGET@83..86 "END"
        "#]],
    );
}

#[test]
fn conditions_and_match() {
    check(
        "if a and not b:\n\tA\nelif c\n\tB\nelse if d\n\tC\nelse:\n\tD\nmatch x\n\twhen \"y\"\n\t\tE\n",
        expect![[r#"
            SOURCE_FILE@0..73
              CONDITION_LINE@0..19
                IF_KW@0..2 "if"
                WHITESPACE@2..3 " "
                EXPR@3..14
                  IDENT@3..4 "a"
                  WHITESPACE@4..5 " "
                  AND_KW@5..8 "and"
                  WHITESPACE@8..9 " "
                  NOT_KW@9..12 "not"
                  WHITESPACE@12..13 " "
                  IDENT@13..14 "b"
                COLON@14..15 ":"
                NEWLINE@15..16 "\n"
                BLOCK@16..19
                  DIALOGUE_LINE@16..19
                    INDENT@16..17 "\t"
                    TEXT@17..18 "A"
                    NEWLINE@18..19 "\n"
              CONDITION_LINE@19..29
                ELIF_KW@19..23 "elif"
                WHITESPACE@23..24 " "
                EXPR@24..25
                  IDENT@24..25 "c"
                NEWLINE@25..26 "\n"
                BLOCK@26..29
                  DIALOGUE_LINE@26..29
                    INDENT@26..27 "\t"
                    TEXT@27..28 "B"
                    NEWLINE@28..29 "\n"
              CONDITION_LINE@29..42
                ELSE_KW@29..33 "else"
                WHITESPACE@33..34 " "
                IF_KW@34..36 "if"
                WHITESPACE@36..37 " "
                EXPR@37..38
                  IDENT@37..38 "d"
                NEWLINE@38..39 "\n"
                BLOCK@39..42
                  DIALOGUE_LINE@39..42
                    INDENT@39..40 "\t"
                    TEXT@40..41 "C"
                    NEWLINE@41..42 "\n"
              CONDITION_LINE@42..51
                ELSE_KW@42..46 "else"
                COLON@46..47 ":"
                NEWLINE@47..48 "\n"
                BLOCK@48..51
                  DIALOGUE_LINE@48..51
                    INDENT@48..49 "\t"
                    TEXT@49..50 "D"
                    NEWLINE@50..51 "\n"
              MATCH_LINE@51..73
                MATCH_KW@51..56 "match"
                WHITESPACE@56..57 " "
                EXPR@57..58
                  IDENT@57..58 "x"
                NEWLINE@58..59 "\n"
                BLOCK@59..73
                  WHEN_LINE@59..73
                    INDENT@59..60 "\t"
                    WHEN_KW@60..64 "when"
                    WHITESPACE@64..65 " "
                    EXPR@65..68
                      STRING@65..68 "\"y\""
                    NEWLINE@68..69 "\n"
                    BLOCK@69..73
                      DIALOGUE_LINE@69..73
                        INDENT@69..71 "\t\t"
                        TEXT@71..72 "E"
                        NEWLINE@72..73 "\n"
        "#]],
    );
}

#[test]
fn mutations() {
    check(
        "do emit(\"coffee\")\nset count += 1\ndo! wait(2)\n$> f()\n$>> g()\n",
        expect![[r#"
        SOURCE_FILE@0..60
          MUTATION_LINE@0..18
            DO_KW@0..2 "do"
            WHITESPACE@2..3 " "
            EXPR@3..17
              IDENT@3..7 "emit"
              L_PAREN@7..8 "("
              STRING@8..16 "\"coffee\""
              R_PAREN@16..17 ")"
            NEWLINE@17..18 "\n"
          MUTATION_LINE@18..33
            SET_KW@18..21 "set"
            WHITESPACE@21..22 " "
            EXPR@22..32
              IDENT@22..27 "count"
              WHITESPACE@27..28 " "
              PLUS_EQ@28..30 "+="
              WHITESPACE@30..31 " "
              NUMBER@31..32 "1"
            NEWLINE@32..33 "\n"
          MUTATION_LINE@33..45
            DO_BANG_KW@33..36 "do!"
            WHITESPACE@36..37 " "
            EXPR@37..44
              IDENT@37..41 "wait"
              L_PAREN@41..42 "("
              NUMBER@42..43 "2"
              R_PAREN@43..44 ")"
            NEWLINE@44..45 "\n"
          MUTATION_LINE@45..52
            DOLLAR_KW@45..47 "$>"
            WHITESPACE@47..48 " "
            EXPR@48..51
              IDENT@48..49 "f"
              L_PAREN@49..50 "("
              R_PAREN@50..51 ")"
            NEWLINE@51..52 "\n"
          MUTATION_LINE@52..60
            DOLLAR2_KW@52..55 "$>>"
            WHITESPACE@55..56 " "
            EXPR@56..59
              IDENT@56..57 "g"
              L_PAREN@57..58 "("
              R_PAREN@58..59 ")"
            NEWLINE@59..60 "\n"
    "#]],
    );
}

#[test]
fn comments_take_the_next_level() {
    check(
        "- a\n\t# about b\n\n\tb\n# about c\nc\n",
        expect![[r##"
        SOURCE_FILE@0..31
          RESPONSE_LINE@0..19
            DASH@0..1 "-"
            WHITESPACE@1..2 " "
            TEXT@2..3 "a"
            NEWLINE@3..4 "\n"
            BLOCK@4..19
              COMMENT_LINE@4..15
                INDENT@4..5 "\t"
                COMMENT@5..14 "# about b"
                NEWLINE@14..15 "\n"
              BLANK_LINE@15..16
                NEWLINE@15..16 "\n"
              DIALOGUE_LINE@16..19
                INDENT@16..17 "\t"
                TEXT@17..18 "b"
                NEWLINE@18..19 "\n"
          COMMENT_LINE@19..29
            COMMENT@19..28 "# about c"
            NEWLINE@28..29 "\n"
          DIALOGUE_LINE@29..31
            TEXT@29..30 "c"
            NEWLINE@30..31 "\n"
    "##]],
    );
}

#[test]
fn inline_parts() {
    check(
        "{{name}}: [[Oi|Olá]] [wave amp=5]tudo[/wave] \\[sic\\] [do x()] [if y]z[/if] [ID:L1] => next [#a]\n",
        expect![[r##"
            SOURCE_FILE@0..97
              DIALOGUE_LINE@0..97
                SPEAKER@0..9
                  REPLACEMENT@0..8
                    L_DBRACE@0..2 "{{"
                    EXPR@2..6
                      IDENT@2..6 "name"
                    R_DBRACE@6..8 "}}"
                  COLON@8..9 ":"
                WHITESPACE@9..10 " "
                INLINE_RANDOM@10..21
                  L_DBRACK@10..12 "[["
                  TEXT@12..14 "Oi"
                  PIPE@14..15 "|"
                  TEXT@15..19 "Olá"
                  R_DBRACK@19..21 "]]"
                WHITESPACE@21..22 " "
                BBCODE@22..34
                  L_BRACK@22..23 "["
                  BBCODE_NAME@23..27 "wave"
                  WHITESPACE@27..28 " "
                  BBCODE_ARGS@28..33 "amp=5"
                  R_BRACK@33..34 "]"
                TEXT@34..38 "tudo"
                BBCODE@38..45
                  L_BRACK@38..39 "["
                  BBCODE_NAME@39..44 "/wave"
                  R_BRACK@44..45 "]"
                WHITESPACE@45..46 " "
                ESCAPE@46..48 "\\["
                TEXT@48..51 "sic"
                ESCAPE@51..53 "\\]"
                WHITESPACE@53..54 " "
                INLINE_MUTATION@54..62
                  L_BRACK@54..55 "["
                  DO_KW@55..57 "do"
                  WHITESPACE@57..58 " "
                  EXPR@58..61
                    IDENT@58..59 "x"
                    L_PAREN@59..60 "("
                    R_PAREN@60..61 ")"
                  R_BRACK@61..62 "]"
                WHITESPACE@62..63 " "
                INLINE_CONDITION@63..69
                  L_BRACK@63..64 "["
                  IF_KW@64..66 "if"
                  WHITESPACE@66..67 " "
                  EXPR@67..68
                    IDENT@67..68 "y"
                  R_BRACK@68..69 "]"
                TEXT@69..70 "z"
                BBCODE@70..75
                  L_BRACK@70..71 "["
                  BBCODE_NAME@71..74 "/if"
                  R_BRACK@74..75 "]"
                WHITESPACE@75..76 " "
                STATIC_ID@76..83
                  L_BRACK@76..77 "["
                  ID_KW@77..79 "ID"
                  COLON@79..80 ":"
                  ID_VALUE@80..82 "L1"
                  R_BRACK@82..83 "]"
                WHITESPACE@83..84 " "
                JUMP@84..91
                  ARROW@84..86 "=>"
                  WHITESPACE@86..87 " "
                  JUMP_TARGET@87..91 "next"
                WHITESPACE@91..92 " "
                TAG_GROUP@92..96
                  L_BRACK@92..93 "["
                  HASH@93..94 "#"
                  TAG@94..95
                    TAG_NAME@94..95 "a"
                  R_BRACK@95..96 "]"
                NEWLINE@96..97 "\n"
        "##]],
    );
}

#[test]
fn random_lines() {
    check(
        "%\n\tA\n%2 [if x] B\n% => c\n",
        expect![[r#"
        SOURCE_FILE@0..24
          RANDOM_LINE@0..5
            RANDOM_PREFIX@0..1
              PERCENT@0..1 "%"
            NEWLINE@1..2 "\n"
            BLOCK@2..5
              DIALOGUE_LINE@2..5
                INDENT@2..3 "\t"
                TEXT@3..4 "A"
                NEWLINE@4..5 "\n"
          DIALOGUE_LINE@5..17
            RANDOM_PREFIX@5..15
              PERCENT@5..6 "%"
              NUMBER@6..7 "2"
              WHITESPACE@7..8 " "
              INLINE_CONDITION@8..14
                L_BRACK@8..9 "["
                IF_KW@9..11 "if"
                WHITESPACE@11..12 " "
                EXPR@12..13
                  IDENT@12..13 "x"
                R_BRACK@13..14 "]"
              WHITESPACE@14..15 " "
            TEXT@15..16 "B"
            NEWLINE@16..17 "\n"
          GOTO_LINE@17..24
            RANDOM_PREFIX@17..19
              PERCENT@17..18 "%"
              WHITESPACE@18..19 " "
            JUMP@19..23
              ARROW@19..21 "=>"
              WHITESPACE@21..22 " "
              JUMP_TARGET@22..23 "c"
            NEWLINE@23..24 "\n"
    "#]],
    );
}

#[test]
fn escapes_and_concurrent() {
    check(
        "\\if this is dialogue\n| Same time: yes\nNo speaker here:\n",
        expect![[r#"
        SOURCE_FILE@0..55
          DIALOGUE_LINE@0..21
            ESCAPE@0..1 "\\"
            TEXT@1..20 "if this is dialogue"
            NEWLINE@20..21 "\n"
          DIALOGUE_LINE@21..38
            PIPE@21..22 "|"
            WHITESPACE@22..23 " "
            SPEAKER@23..33
              TEXT@23..32 "Same time"
              COLON@32..33 ":"
            WHITESPACE@33..34 " "
            TEXT@34..37 "yes"
            NEWLINE@37..38 "\n"
          DIALOGUE_LINE@38..55
            TEXT@38..54 "No speaker here:"
            NEWLINE@54..55 "\n"
    "#]],
    );
}

#[test]
fn imports_and_using() {
    check(
        "import \"res://a.dialogue\" as a\nusing SomeState\n=>< a/start\n",
        expect![[r#"
        SOURCE_FILE@0..59
          IMPORT_LINE@0..31
            IMPORT_KW@0..6 "import"
            WHITESPACE@6..7 " "
            STRING@7..25 "\"res://a.dialogue\""
            WHITESPACE@25..26 " "
            AS_KW@26..28 "as"
            WHITESPACE@28..29 " "
            IDENT@29..30 "a"
            NEWLINE@30..31 "\n"
          USING_LINE@31..47
            USING_KW@31..36 "using"
            WHITESPACE@36..37 " "
            IDENT@37..46 "SomeState"
            NEWLINE@46..47 "\n"
          GOTO_LINE@47..59
            JUMP@47..58
              ARROW_RETURN@47..50 "=><"
              WHITESPACE@50..51 " "
              JUMP_TARGET@51..58 "a/start"
            NEWLINE@58..59 "\n"
    "#]],
    );
}

#[test]
fn bom_crlf_and_trailing_space() {
    check(
        "\u{feff}~ a \r\nLine\t\r\n",
        expect![[r#"
        SOURCE_FILE@0..16
          BOM@0..3 "\u{feff}"
          TITLE_LINE@3..9
            TILDE@3..4 "~"
            WHITESPACE@4..5 " "
            TITLE_NAME@5..6 "a"
            WHITESPACE@6..7 " "
            NEWLINE@7..9 "\r\n"
          DIALOGUE_LINE@9..16
            TEXT@9..13 "Line"
            WHITESPACE@13..14 "\t"
            NEWLINE@14..16 "\r\n"
    "#]],
    );
}

#[test]
fn bad_indentation_still_round_trips() {
    check(
        "\t\tA\nB\n\t\t\tC\n\tD\n",
        expect![[r#"
        SOURCE_FILE@0..14
          DIALOGUE_LINE@0..4
            INDENT@0..2 "\t\t"
            TEXT@2..3 "A"
            NEWLINE@3..4 "\n"
          DIALOGUE_LINE@4..14
            TEXT@4..5 "B"
            NEWLINE@5..6 "\n"
            BLOCK@6..14
              DIALOGUE_LINE@6..11
                INDENT@6..9 "\t\t\t"
                TEXT@9..10 "C"
                NEWLINE@10..11 "\n"
              DIALOGUE_LINE@11..14
                INDENT@11..12 "\t"
                TEXT@12..13 "D"
                NEWLINE@13..14 "\n"
    "#]],
    );
}
