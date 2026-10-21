# Formats

dialogue-maker writes a project's script in other formats (exports), and turns other tools' scripts into Dialogue Manager files (imports). From the command line:

```sh
dialogue-maker export json        path/to/game -o script.json
dialogue-maker export translation path/to/game -o translations.csv --locale en --translate-to pt_BR
dialogue-maker export voice       path/to/game -o voice.md --character keeper
dialogue-maker export voice-csv   path/to/game -o recording.csv
dialogue-maker export fountain    path/to/game -o script.fountain

dialogue-maker import story.twee -o dialogue/story.dialogue
```

Without `-o`, the result goes to standard output. In the app, **Export…** and **Import…** in the toolbar do the same.

Exports read the script as it is in the app, unsaved edits included. Imports write a new file only: never over one that exists.

## Exports

### JSON (`json`)

The whole script for other tools: a build step, a localisation service, a wiki. Its format is documented and versioned, unlike the app's own view data:

- `format` is always `"dialogue-maker/script"` and `version` is `1`. New fields can appear in any version; the version goes up when a field changes meaning or goes away.
- The full description is a JSON Schema: [`script.schema.json`](script.schema.json). It is generated from the Rust types, and a test fails when it is out of date.

At the top: `name`, `characters`, `places`, `flags` (each with its meaning and the labels that set and read it), and `files`. Each file has a `preamble` (what plays before its first label) and its `labels`. Each label has an `id`, its `line` in the file, its `place` and `section`, the comments before it (`notes`), and a `body` of nodes.

Every node has a `type` and a `line` (in its file, from 1), plus the comments right before it (`notes`) and authorship `marks` when there are any:

| `type` | Fields | Dialogue Manager |
|---|---|---|
| `line` | `speaker`, `character` (the id it belongs to), `text` (as written, markup included), `plain` (without markup), `tags` (`name`, `value`), `id`, `concurrent`, `jump` | `Keeper: Hello. [#mood=calm] [ID:K1]` |
| `menu` | `choices`: each with `line`, `text`, `plain`, `speaker`, `tags`, `id`, `condition`, `jump`, `body` | `- Leave [if has_key]` |
| `branch` | `arms`: each with `line`, `keyword` (`if`, `elif`, `else`), `condition`, `body` | `if` / `elif` / `else` |
| `match` | `subject`, `arms` (`keyword` `when`, or `else`) | `match` / `when` |
| `while` | `condition`, `body` | `while` |
| `random` | `options`: each with `line`, `weight`, `condition`, `body` | `%` lines |
| `effect` | `keyword` (`do`, `set`...), `expression` | `do x()`, `set y = 1` |
| `jump` | `to` (`label`, `end`, `end_conversation`, `expression`), `label` or `expression`, `returns` | `=> x`, `=> END`, `=> END!`, `=>< x` |
| `anchor` | `id` | a title inside a block |

### Translation table (`translation`)

The CSV that Dialogue Manager's own export writes and Godot imports: a `keys` column, a column for the script's locale (`--locale`, `en` by default), one per translation (`--translate-to`, again for more), then `_character` (who says it; `(response)` for a choice) and `_notes` (the `##` comments above the line). A line's key is its static id (`[ID:...]`), or else its text, as in Dialogue Manager.

Exporting over a table that exists keeps it:

- its delimiter (`,`, `;` or tab) and columns;
- the translations of every line still in the script, with the script's current text in the source column;
- the rows of lines no longer in the script, after the others, so no translator's work is lost.

### Voice script (`voice`) and recording sheet (`voice-csv`)

For voice actors: each character's lines, grouped by the mood to read them in, with the line before each one for context, its label, file and line, and its static id (which names the audio file in Dialogue Manager). The mood is the value of the tag the project file shows as `voice`; a character's moods come in the order of their `voices`, then the rest, then lines with no mood. Lines said in thought (the tag shown as `thought`) say so.

`voice` is Markdown, one section per character (`--character` for one); `voice-csv` is one row per line to sort and fill in at a recording session.

### Screenplay (`fountain`)

The script in [Fountain](https://fountain.io), which screenwriting apps (Highland, Fade In, Slugline, Beat) open and print. Each label is a scene, each file a section; spoken lines are dialogue under the character's name, with the mood (and thinking, shaking) as a parenthetical; narration is action; concurrent lines are dual dialogue. Choices, branches and jumps, which a screenplay has no form for, are action lines starting with `—` or `→`; game code is in notes (`[[ ]]`).

## Imports

Each importer converts what has a match in Dialogue Manager. What does not is kept in the file as a comment, and listed as a warning with its line in the source, so nothing is dropped without a word. Titles are made from the source's names (letters, digits and `_`; accents dropped).

A line reads as spoken (`Name: words`) when what is before the colon looks like a name: up to three words, each capitalised. Any other colon in narration is escaped (`\:`) so Dialogue Manager does not read a speaker into it.

### Twine (`.twee`, `.tw`)

Twee 3, which Twine 2 exports (Library → Archive → Twee, or Build → Export as Twee). Each passage is a title; its lines are narration or spoken lines; its links (`[[Text->Target]]`, `[[Text|Target]]`, `[[Target<-Text]]`, `[[Target]]`) are choices after its text. A passage with no links ends there (`=> END`). Variables printed in the text (`$gold`) become `{{gold}}`, and `<<set $x to 1>>` (SugarCube) or `(set: $x to 1)` (Harlowe) become `set x = 1`. Other macros, and passages tagged `script`, `stylesheet` or `widget`, are left out with a warning.

### Yarn Spinner (`.yarn`)

Yarn Spinner 2. Each node is a title. Lines keep their speaker; `#line:` ids become `[ID:...]` and other hashtags tags; `-> options` become choices, and `<<if>>` on one its condition; `<<if>>` / `<<elseif>>` / `<<else>>` / `<<endif>>` become `if` blocks; `<<set>>`, `<<jump>>`, `<<detour>>`, `<<return>>` and `<<stop>>` their Dialogue Manager forms; `{$x}` becomes `{{x}}`, and `$` goes from variable names. Yarn's word operators (`is`, `gt`, `lte`...) become symbols. `<<declare>>`, `<<wait>>` and custom commands are kept as comments: declare the variables in the game's state.

### Ink (`.ink`)

Knots and stitches become titles (`knot_stitch`); the weave (choices `*` and `+`, gathers `-`, nested by their count) becomes indented choices, with `[brackets]` deciding what the choice shows and what is said after it, as in Ink. Diverts become jumps: `-> DONE` ends the title, `-> END` the conversation, a tunnel `-> x ->` jumps and comes back. `~` logic becomes `set` or `do`; conditional blocks become `if` / `elif` / `else` or `match` / `when`; inline `{x}`, `{cond: a|b}` and `{~a|b}` become `{{x}}`, `[if cond]a[else]b[/if]` and `[[a|b]]`. Labels on choices and gathers become titles to jump to.

What Ink has and Dialogue Manager does not is kept as a comment with a warning: functions, threads (`<-`), fallback choices, `VAR` and `CONST` (declare them in the game's state). Choices offered once only (`*`) are offered every time, sequences (`{a|b}`) pick at random, and glue (`<>`) is dropped.
