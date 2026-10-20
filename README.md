# dialogue-maker

A desktop workbench for game scripts. It opens a game project's own dialogue files where they live, shows the whole script, edits the files back without disturbing anything it was not asked to change, and ships the change as a pull request. Offline; Mac, Windows and Linux.

First format: [Dialogue Manager](https://github.com/nathanhoad/godot_dialogue_manager) 3 `.dialogue` files (Godot 4).

**Status:** M4. The app opens a project, shows its whole script, and edits it: lines, choices, speakers, faces, tags and conditions, written back only on save and only where they changed. The command line checks the script, counts it, and writes it as one HTML page. Plan, decisions and milestones: [`PLAN.md`](PLAN.md).

Working name; the public name can change before release.

## Layout

```
crates/
  dm-syntax/   lossless syntax tree for .dialogue
  core/        the script model, the project file, checks, stats
  cli/         `dialogue-maker` command line
app/
  src-tauri/   Tauri 2 desktop app over core
  ui/          Svelte 5 + Vite frontend
```

## Use

The app: open a game project's folder (or start it with one, `dm-app path/to/game`). It shows the script by place, with search, the project's variant filters, faces, links between labels, its documents, a flags index and who talks; it reloads when a file changes on disk.

Turn on **Edit** to change it: open a line or choice (✎, or double-click) to change its speaker, words, face, voice, thought or shake, or a choice's condition, with a preview of the balloon; add lines and choices, move them, remove them; edit `if`/`when` conditions with the project's flags at hand. Edits stay in the app until **Save**, which shows the change to each file first (**Review**), writes only the files you changed, and only the lines you changed in them. A file that changed on disk since it was read (the Godot editor saved it, a `git pull`) is never written over: the save is refused, your edits are kept, and you choose. Undo and redo with Ctrl+Z and Ctrl+Shift+Z; save with Ctrl+S.

The command line:

```sh
dialogue-maker check path/to/game     # problems, compiler style; exit 1 on errors
dialogue-maker stats path/to/game     # labels, lines, choices, speakers, tags
dialogue-maker build path/to/game -o script.html   # the app's read view as one offline page
```

All read `dialogue-maker.toml` at the game's root, or the file given with `--config`. See [the project file](docs/project-file.md).

Installers and command-line builds for the three systems come from the release workflow (a `v*` tag, or a manual run). They are not signed yet.

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
