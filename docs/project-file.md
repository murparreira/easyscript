# The project file

`dialogue-maker.toml`, at the root of the game's repository, holds what a project knows about itself that its dialogue files cannot say: which files are dialogue, who the speakers are, what the tags mean. Every field is optional. Without the file, dialogue-maker reads every `*.dialogue` outside `addons/` and skips the checks that need it.

Longer documents (a walkthrough, review notes) go in a `.dialogue-maker/` folder next to it.

Paths of sources, portraits and assets are relative to the project root. Paths of documents are relative to the project file's own folder. A key the format does not know is an error, so a typo does not go unnoticed.

```toml
name = "Lighthouse"

# The comment that opens a section of labels. The group named `title` (or the first
# group) is the section's title.
section_header = '^# =+ (?P<title>.*?) *=*$'

[sources]
dialogue = ["dialogue/**/*.dialogue"]   # default: ["**/*.dialogue"]
exclude = ["dialogue/debug/**"]         # default: ["addons/**"]
# The game's code and scenes. A label named there in quotes is an entry point, and so is
# a label matching a format string there ("gull_%d", "{npc}_talk").
code = ["src/**/*.gd", "src/**/*.tscn"]

[checks]
# Labels the game starts by a name it builds at run time, as globs.
entry_points = ["*_generic_p[0-9]"]
# Check codes not to report.
ignore = []

# Places in the order a player meets them. A place takes whole files, or only some of a
# file's labels (`labels`), or all but some (`exclude_labels`).
[[places]]
id = "pier"
name = "The Pier"
group = "Harbour"
files = ["dialogue/pier.dialogue"]

# Characters in the order they are listed. `speakers` are the names the dialogue files use.
[[characters]]
id = "keeper"
name = "The Keeper"
speakers = ["Keeper", "Old Man"]
portrait = "faces/keeper_calm.png"
role = "Keeps the lamp lit; knows every ship that sank."
voices = ["calm", "angry"]   # voice moods recorded; a line asking for another is reported

# Line tags, `[#name=value]`. Kinds: `flag` (no value), `enum` (one of `values`),
# `asset` (a file, at `path` with `{value}`), `text` (anything).
# `show` tells the views how to show it: thought, shake, voice, face.
[tags.face]
kind = "asset"
path = "faces/{value}.png"
show = "face"

[tags.mood]
kind = "enum"
values = ["calm", "angry"]
show = "voice"

[tags.thought]
kind = "flag"
show = "thought"

# Conditions the views can filter on.
[[variants]]
name = "Difficulty"
condition = "Globals.easy_mode()"
true = "Easy"
false = "Hard"

# A comment with the marker marks the next line (or the label, right under its title)
# as someone's. Text after the marker is the mark's note.
[[marks]]
marker = "✏ prod"
who = "production"

# Flags: which functions set and read them, and what they mean.
[flags]
set = ["set_flag"]
read = ["has_flag"]
match_subjects = ["Globals.current_lead()"]   # `when` values under these are flags read
meanings = { oil_found = "The oil was found in the shed." }

# Markdown documents. `[[label]]` links into the script and is checked.
[[documents]]
path = ".dialogue-maker/walkthrough.md"
title = "Walkthrough"
```

## Checks

`dialogue-maker check` reports, as `path:line:column: severity[code]: message`:

| Code | Severity | What |
|---|---|---|
| `unreadable-file` | error | A dialogue file that is not UTF-8. It is left alone. |
| `syntax` | error | Text the parser could not read (in an expression, mostly). |
| `duplicate-label` | error | Two titles with the same name in a file. |
| `unknown-jump-target` | error | A jump to a title the file does not have (or import). |
| `unknown-speaker` | warning | A speaker no character lists. Needs `[[characters]]`. |
| `missing-portrait` | error | A character's portrait file does not exist. |
| `missing-voice` | warning | A line asks for a voice mood (the tag shown as `voice`) its speaker has no recording in. Needs `voices`. |
| `unknown-tag` | warning | A tag not in `[tags]`. Needs `[tags]`. |
| `bad-tag-value` | warning | A value a tag's kind does not allow. |
| `missing-asset` | error | An `asset` tag's file does not exist. |
| `unreachable-label` | warning | Nothing jumps or falls into a label, no code names it, no entry point matches. Needs `sources.code` or `checks.entry_points`. |
| `unknown-place-file`, `unknown-place-label` | error, warning | A place names a file or label that is not there. |
| `missing-document`, `broken-link` | error | A document that cannot be read, a `[[label]]` link to no label. |

It exits with 1 when there are errors (or warnings, with `--deny-warnings`), 2 when the project does not load.
