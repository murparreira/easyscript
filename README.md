# dialogue-maker

A desktop workbench for game scripts. It opens a game project's own dialogue files where they live, shows the whole script, edits the files back without disturbing anything it was not asked to change, and ships the change as a pull request. Offline; Mac, Windows and Linux.

First format: [Dialogue Manager](https://github.com/nathanhoad/godot_dialogue_manager) 3 `.dialogue` files (Godot 4).

**Status:** M9. The app opens a project (or clones it from GitHub), shows its whole script, edits it and plays it: lines, choices, speakers, faces, tags, conditions, jumps and scenes, written back only on save and only where they changed. Its Project tab edits the project file itself (characters, tags, flags, places, variants), keeping its comments, and drafts one for a project that has none. It commits the script to git and opens the pull request on GitHub. It exports the script for other tools, translators, voice actors and screenwriting apps, and imports Twine, Yarn Spinner and Ink scripts. The command line checks the script, counts it, and writes it as one HTML page. Plan, decisions and milestones: [`PLAN.md`](PLAN.md).

Working name; the public name can change before release.

## Layout

```
crates/
  dm-syntax/   lossless syntax tree for .dialogue
  core/        the script model, the project file, checks, stats, edits, git
  formats/     exports (JSON, translation CSV, voice scripts, Fountain), imports (Twine, Yarn, Ink)
  github/      GitHub sign-in, push, pull requests
  cli/         `dialogue-maker` command line
app/
  src-tauri/   Tauri 2 desktop app over core
  ui/          Svelte 5 + Vite frontend
```

## Use

The app: open a game project's folder, or **Open from GitHub** (sign in, pick one of your repositories, and the app clones it into a folder and opens it). It reads the dialogue files where they are, follows them as they change on disk, and writes only the files you change, when you save. The bar at the top switches between four views:

- **Write**: the scenes (labels) place by place, searchable; the scene as line cards, each with the speaker's face, how it is said (voice, thought, shake, face) and its words. Click a line to change it, with a preview of the balloon; add lines, choices and jumps to other scenes; move or remove them; edit `if` and `when` conditions with the project's flags at hand; add and remove scenes. Beside it, **Play** runs the script as the game would, from the scene or one you choose: lines one at a time, choices to pick, flags and variables tracked. What only the game knows (its own functions) it asks once and remembers, and a variant's question is asked by its sides' names.
- **Read**: the whole script as one document, with its documents, a flags index, who talks and the problems; searchable, filtered by the project's variants.
- **Map**: labels place by place with the jumps, choices and flags between them, where the game starts them and which nothing reaches; grids of who speaks where and which flags each place sets and reads.
- **Project**: the project file's builders, a tab each: characters, tags, flags, places, variants; or, for a project with none, a draft from the dialogue.

Edits stay in the app until **Save**: the bar says what is not saved, and shows the change to each file before saving. A file that changed on disk since it was read (the Godot editor saved it, a `git pull`) is never written over: the save is refused, your edits are kept, and you choose. Undo and redo with Ctrl+Z and Ctrl+Shift+Z; save with Ctrl+S.

**Share** (the branch in the bar) shows the branch, makes and switches branches, and commits the project's own files (dialogue, project file, documents) as saved, and nothing else: what the team staged by hand stays staged. Conflicts are shown, never resolved. When `origin` is on GitHub, **Sign in to GitHub** shows a code; GitHub, in your browser, asks you to authorize dialogue-maker and to enter it (GitHub's device flow: no password reaches the app). The app keeps the sign-in in the system's password store (Keychain, Credential Manager, Secret Service), or only until it closes when there is none; revoke it any time in GitHub's settings under Applications. **Push and open pull request**, on a branch with everything committed, pushes the branch (never forced: if GitHub has commits you do not, you are told) and opens its pull request, or finds the one already open. A folder that is not in git has nowhere to push: open the game from GitHub instead. A repository with no `origin` can be linked to one of your repositories. Nothing reaches GitHub until you ask.

**Export** has the dialogue files for the game first: each file's status and a look at it, saved in place, or as a ZIP that keeps the game's folders (it unzips over the project). Then the script for others: Dialogue Manager's translation table (exporting over a table keeps its translations), voice scripts and recording sheets by character and mood, a Fountain screenplay, and JSON for other tools (a documented format). **Import** turns a Twine (`.twee`), Yarn Spinner (`.yarn`) or Ink (`.ink`) script, or a `.dialogue` file from elsewhere, into a new file in the project, after showing it and what did not convert. See [the formats](docs/formats/README.md).

**Settings**: the app's language (English or Portuguese (Brazil); it follows the system's), light or dark (or like the system), and what the script shows. A project file can name the game's fonts (`[look]`), so the script reads as it does in the game; the app's own controls keep the system's font. Voice scripts, recording sheets and screenplays are written with their own words (headings, stage directions) in the app's language (`--language pt-BR` on the command line). The command line and import warnings are in English for now.

The command line:

```sh
dialogue-maker check path/to/game     # problems, compiler style; exit 1 on errors
dialogue-maker stats path/to/game     # labels, lines, choices, speakers, tags
dialogue-maker build path/to/game -o script.html   # the app's read view as one offline page
dialogue-maker init path/to/game      # a first dialogue-maker.toml, drafted from the dialogue
dialogue-maker export translation path/to/game -o translations.csv --translate-to pt_BR
dialogue-maker export zip path/to/game -o dialogue.zip   # the dialogue files themselves
dialogue-maker import story.ink -o path/to/game/dialogue/story.dialogue
```

All read `dialogue-maker.toml` at the game's root, or the file given with `--config`. See [the project file](docs/project-file.md).

Installers and command-line builds for the three systems come from the release workflow (a `v*` tag, or a manual run). They are not signed yet: see [releasing](docs/release.md) for signing, updates and the public name.

## Develop

You need Rust stable (pinned by `rust-toolchain.toml`), Node 22, and [Tauri's system dependencies](https://tauri.app/start/prerequisites/). With [mise](https://mise.jdx.dev), `mise install` sets up Rust and Node.

```sh
npm ci                   # frontend and Tauri CLI
npm run dev              # the app, live-reloading
npm run check            # type-check the frontend
npm test                 # frontend tests
cargo test --workspace   # Rust tests; also writes the frontend's model types
npm run build:page       # the single-file page `dialogue-maker build` embeds
cargo run -p dm-cli      # the CLI (build the page first for `build`)
```

The app's own words go through `t("English text")` (`app/ui/src/lib/i18n.svelte.ts`); translations live in `app/ui/src/lib/locales/`, and `npm test` fails when a message has none or a translation is left over.

The frontend's types in `app/ui/src/lib/bindings/` are generated from the Rust model by `cargo test` (ts-rs); commit them with the model change. To work on the read view in a browser, write a view with `dialogue-maker build path/to/game --json -o app/ui/dev/view.json` and open the Vite dev server (`npm run dev -w app/ui`).

Tests that read a real game project take its path from `DIALOGUE_MAKER_PROJECT` and skip when it is unset:

```sh
DIALOGUE_MAKER_PROJECT=../na-cabeca-do-bicho cargo test --workspace
```

No team's script is committed here; public tests use fixtures written for them.

### Linux with NVIDIA

WebKitGTK's DMA-BUF renderer crashes on the NVIDIA driver under Wayland (`Error 71 (Protocol error) dispatching to Wayland display`). The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself when it finds the NVIDIA driver and the variable is unset; set it to `0` to opt out.

## Licence

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
