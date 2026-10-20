# dialogue-maker

A desktop workbench for game scripts. It opens a game project's own dialogue files where they live, shows the whole script, edits the files back without disturbing anything it was not asked to change, and ships the change as a pull request. Offline; Mac, Windows and Linux.

First format: [Dialogue Manager](https://github.com/nathanhoad/godot_dialogue_manager) 3 `.dialogue` files (Godot 4).

**Status:** M2. The command line checks and counts a project's script; the app does not open a project yet. Plan, decisions and milestones: [`PLAN.md`](PLAN.md).

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

```sh
dialogue-maker check path/to/game     # problems, compiler style; exit 1 on errors
dialogue-maker stats path/to/game     # labels, lines, choices, speakers, tags
```

Both read `dialogue-maker.toml` at the game's root, or the file given with `--config`. See [the project file](docs/project-file.md).

## Develop

You need Rust stable (pinned by `rust-toolchain.toml`), Node 22, and [Tauri's system dependencies](https://tauri.app/start/prerequisites/). With [mise](https://mise.jdx.dev), `mise install` sets up Rust and Node.

```sh
npm ci                   # frontend and Tauri CLI
npm run dev              # the app, live-reloading
npm run check            # type-check the frontend
cargo test --workspace   # Rust tests
cargo run -p dm-cli      # the CLI
```

Tests that read a real game project take its path from `DIALOGUE_MAKER_PROJECT` and skip when it is unset:

```sh
DIALOGUE_MAKER_PROJECT=../na-cabeca-do-bicho cargo test --workspace
```

No team's script is committed here; public tests use fixtures written for them.

### Linux with NVIDIA

WebKitGTK's DMA-BUF renderer crashes on the NVIDIA driver under Wayland (`Error 71 (Protocol error) dispatching to Wayland display`). The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself when it finds the NVIDIA driver and the variable is unset; set it to `0` to opt out.

## Licence

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
