# dialogue-maker: plan

A desktop workbench for game scripts. It opens a game project's own dialogue files where they live, shows the whole script three ways (the story at a glance, a writing view, a reading view), edits the files back without disturbing anything it was not asked to change, and ships the change as a pull request. Installable, offline, Mac, Windows and Linux. Open source once it stands on its own.

It grows out of `tools/gerar_documentos` in *Na Cabeça do Bicho* (Godot 4, Dialogue Manager 3.10.3), which builds a static script page from the game's files. That game is the first user and the reference project; it stays a separate repo (`../na-cabeca-do-bicho`).

Working name; the public name can change before release.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Source of truth | The project's files in its repo | Teams with an existing project adopt it with no import step; a PR is a small readable diff; the app can be dropped at any time |
| Editing | Lossless: only the edited span changes on disk | Comments, authorship marks, header blocks, blank lines and indentation are part of the script |
| GUI | Tauri 2 (Rust backend, the OS webview for the UI) | Installable on all three systems, no server, ~10 MB; reuses the current page's HTML/CSS/JS ideas; CodeMirror for editing; graph libraries for the macro view |
| Repo | Sibling folder, its own git repo | No Rust build output or app history in the game repo; the game is treated like any other team's project |
| First milestone | Lossless parser, then the read view | Reading proves the model on a real script before anything writes to disk |
| Frontend | Svelte 5 + Vite, npm (confirmed at M0) | Small runtime for a view that mostly renders one large model |
| Licence | MIT OR Apache-2.0 | The Rust norm |
| Typed bridge | ts-rs (confirmed at M3) | specta 2 is still a release candidate; ts-rs is stable and covers the model's serde shapes. Commands are few, typed by hand |
| Faces in the view | Thumbnails as data URIs in the view's data | One input for the app and the static page; no asset protocol to scope |

## Rules

1. **Never lose a byte.** `print(parse(file)) == file` for every file, always. Syntax the parser does not understand becomes an opaque node kept verbatim, never dropped or normalised.
2. **Never write a file the user did not change**, and never write over a file that changed on disk since it was read (the Godot editor rewrites `.dialogue` files while open): compare a hash, refuse, reload.
3. **No game-specific code in the core.** What a project knows about itself lives in its project file (below). Readers of one engine's or one game's data (a Godot `.tres`, a GDScript registry) are adapters behind a trait.
4. **No team's script in this repo.** Public tests use fixtures written for them. Tests on a real project read it from a path in an environment variable (`DIALOGUE_MAKER_PROJECT=../na-cabeca-do-bicho`) and skip when it is unset; CI never has it.
5. **Offline.** Nothing leaves the machine except what the user sends (a git push, a PR). No telemetry.

## Architecture

Cargo workspace:

```
crates/
  dm-syntax/   lossless syntax tree (rowan) for Dialogue Manager .dialogue; the first format adapter
  core/        (package `dm-core`: `core` is Rust's own) the model (project, characters, scenes, lines, choices, conditions, effects, tags),
               built from syntax trees; validation; stats; edits expressed as syntax-tree edits
  cli/         `dialogue-maker check | stats | build | export` (build = the read view as one static HTML file)
app/
  src-tauri/   Tauri 2 commands over core; file watching (notify)
  ui/          TypeScript frontend (Svelte 5 + Vite)
```

- **One renderer, two hosts.** The read view is a frontend component fed the model as JSON. The app renders it live; `cli build` bundles the same frontend and the JSON into one HTML file (what `roteiro.py` makes today). No second renderer in Rust.
- **Typed bridge.** The model's types are generated for TypeScript from Rust (ts-rs, by `cargo test`), so the frontend cannot drift from the core; CI fails when the committed types are stale.
- **Formats as adapters.** A format reads a file into the model's syntax-backed scene tree and applies edits back to its own syntax tree. `.dialogue` first; others later (see M8).

### The project file

`dialogue-maker.toml` at the game repo's root (a folder `.dialogue-maker/` for longer documents). It holds what the dialogue files cannot, and replaces `roteiro_conteudo.py`:

- **Sources:** dialogue globs, and how a file maps to places (one file per place; a place can take part of a file, as the Edifício Mestre's hall and 25th floor split one file today).
- **Characters:** id, display name, speaker names used in the files (`Vizinho` → `cavalo`), portrait path, a reaction pattern (`assets/reactions/{stem}.png`), short role.
- **Tag schema:** each line modifier, its kind and values, and how the views show it. `pensamento` (bool, thought bubble), `tremor` (bool, shake), `voz` (enum: serio, feliz, chateado, irritado, sonolento, mudo), `reaction` (asset reference).
- **Variants:** a condition the views can filter on, with a name. `Globals.modo_assistido()` → Assistido / Detetive.
- **Authorship marks:** a comment marker and who it means. `✏ produção` → production.
- **Block headers:** the comment pattern that opens a character's block (`# === NAME (...) ===`).
- **Flags:** name → meaning.
- **Places:** id, name, group (district), order.
- **Documents:** Markdown files for the walkthrough and review notes, with `[[label]]` links checked like any other.

For *Na Cabeça do Bicho* the first version is generated once from `roteiro_conteudo.py` and the CharacterRegistry, then edited by hand. It lives in the game repo.

## The read view (milestone 1 target)

What today's page has, sorted by where it goes:

| Today | In dialogue-maker |
|---|---|
| The script place by place: labels, lines, choices, conditions, effects, tags, portraits, `[[label]]` and `=>` links | core |
| Search with highlight, ids toggle, authorship marks toggle, deep links, table of contents | core |
| Mode filter (Assistido / Detetive) | core, generalised to variants |
| Flags index (who sets, who reads), who-talks stats | core |
| How to read, the story walkthrough, review notes | project documents (Markdown) |
| The notebook (items, map notes), minigames, comic pages | game-specific; left out of M3. The Python tool keeps making them until a plugin or document system covers them |

**Parity check:** on *Na Cabeça do Bicho*, the same counts as `roteiro.py` prints (labels, lines, choices, speakers) and zero broken links.

## Milestones

Each ends with something a person can use. No editing before the read view matches today's page.

- **M0 Scaffold.** Workspace, licence, README, CI on three systems (fmt, clippy, test), a Tauri window that opens on Linux. Confirm the frontend framework.
- **M1 Lossless parser.** `dm-syntax` covers what Dialogue Manager 3.x accepts, starting with what the reference game uses: labels (211), jumps (232), choices (51), `if`/`elif`/`else`, `match`/`when`, `do` (85), inline `[if]`, tags (764 lines), `[[a|b]]`, comments (367). Not used there but in the grammar: `set`, `=><`, `{{ }}`, `%` random lines, `while`, `import`/`using`, BBCode tags. Anything else is opaque. Never panics on bad input. Tests: round trip on fixtures, a property test (any byte string round-trips), and the reference game's 18 files through the environment variable.
- **M2 Model, checks, CLI.** `core` builds the model; `check` reports unknown jump targets, unknown speakers, missing portraits or reactions, tags outside the schema, unreachable labels; `stats` prints the counts. Parity numbers above.
- **M3 Read view.** Open a project folder in the app; the script by place; search, filters, links, portraits; live reload when a file changes on disk; `cli build` writes the static page. Unsigned installers from CI for all three systems. *End of the first milestone: the team reads the script in the app, and `tools/gerar_documentos/roteiro.py` can retire.*
- **M4 Editor.** Change a line's text, speaker, reaction and tags; add, remove and reorder lines and choices; edit conditions against known flags. Every change is a syntax-tree edit; a diff panel shows the file change before saving; undo. Live balloon preview (thought bubble, shake).
- **M5 Builders.** Characters (portrait, reaction gallery, voice moods), the tag schema with previews, flags, places, variants; all saved to the project file. An importer for a project with no project file yet (scan speakers, tags, flags).
- **M6 Macro view.** The story at a glance: label flow from jumps and conditions, flags (where set, where read), a character × place grid, the walkthrough beside it.
- **M7 Git and GitHub.** Branch, commit and diff from the app (`git2`); open a PR (GitHub API, device-flow login). Conflicts are shown, never auto-resolved.
- **M8 Formats.** Export JSON (documented schema), CSV (translation; a voice-actor script per character and mood), Markdown or Fountain screenplay. Import Yarn Spinner, Ink, Twee.
- **M9 Release.** Signing (Apple notarisation, Windows certificate), updater, docs, translations of the app's own UI (English first, pt-BR), the public name, extraction checklist.

## Risks

- **Grammar drift** between Dialogue Manager versions: the grammar is versioned; the addon's own compiler (in the reference game, `addons/dialogue_manager/compiler/`) is the reference when in doubt.
- **Webview differences**, WebKitGTK on Linux most of all: test Linux from M0; no CSS or JS that WebKitGTK lacks without a fallback (today's page uses the CSS Highlight API for search).
- **Scope.** This is a large product. The milestone order is the guard: read, then edit, then the rest.
- **Two editors on one file** (the Godot editor and this app): rule 2.

## Open, not blocking M0 to M3

Public name (and the app identifier, `com.busyboys.dialogue-maker` for now). Whether the art list (`lista_arte`) belongs in the app. Plugin mechanism for game-specific sections (scripted, or a documented JSON a game's own tool writes).

## How we work

- Sessions run in this folder with the game as a reference directory: `cd ../dialogue-maker && claude --add-dir ../na-cabeca-do-bicho`. The game repo is read, not changed, until the project file goes in at M5 (or earlier, by hand, as a separate change in that repo).
- Reference code in the game repo: `tools/gerar_documentos/` (`dialogo.py` parser, `dados.py` game readers, `roteiro.py` page, `roteiro_conteudo.py` hand-written data, `roteiro.html` template and page script).
