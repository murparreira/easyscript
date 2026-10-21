//! The draft project file for the lighthouse fixture, from its dialogue alone.

use std::path::Path;

use expect_test::expect;

#[test]
fn draft_from_the_dialogue() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lighthouse");
    let draft = es_core::init::draft(&root).expect("drafts");
    // The draft is a valid project file.
    es_core::Config::parse(&draft)
        .expect("parses")
        .compile()
        .expect("compiles");
    expect![[r##"
        # easyscript project file, drafted by `easyscript init` from the dialogue files.
        # Every value below is a guess from what the files say: check it, then edit freely.
        # The format: https://github.com/murparreira/easyscript/blob/main/docs/project-file.md

        name = "lighthouse"

        [sources]
        dialogue = ["**/*.dialogue"]
        exclude = ["addons/**"]
        # The game's code, so that labels it names count as reached: code = ["src/**/*.cs"]

        # One character per speaker name, most lines first. Join speakers that are the same
        # character into one `speakers` list, and add a `portrait`.
        [[characters]]
        id = "keeper"
        name = "Keeper"
        speakers = ["Keeper"]
        # 11 lines

        [[characters]]
        id = "gull"
        name = "Gull"
        speakers = ["Gull"]
        # 1 line

        [[characters]]
        id = "sailor"
        name = "Sailor"
        speakers = ["Sailor"]
        # 1 line

        [[characters]]
        id = "stranger"
        name = "Stranger"
        speakers = ["Stranger"]
        # 1 line

        [tags.face]
        kind = "enum"
        values = ["keeper_calm", "sailor_worried"]
        # on 6 lines

        [tags.mood]
        kind = "enum"
        values = ["calm", "sleepy"]
        # on 2 lines

        [tags.thought]
        kind = "flag"
        # 1 use with a value, which a flag does not take
        # on 2 lines

        [tags.volume]
        kind = "enum"
        values = ["11"]
        # on 1 line

        [flags]
        # Calls with "flag" in their name and a string first; by their verb, which set and which read.
        set = ["set_flag"]
        read = ["has_flag"]

        # Still to add by hand: places, variants, authorship marks, documents. See the format.
    "##]].assert_eq(&draft);
}
