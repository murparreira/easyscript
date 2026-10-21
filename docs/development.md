# Developing easyscript

How the code is laid out, how to build and test it, and the conventions. [`PLAN.md`](../PLAN.md) has the plan, the decisions and the rules; [releasing](release.md) has signing, updates and the name.

## Layout

```
crates/
  syntax/      lossless syntax tree for .dialogue (`es-syntax`; the others are `es-core`, `es-cli`…)
  core/        the script model, the project file, checks, stats, edits, git and the team
  formats/     exports (JSON, translation CSV, voice scripts, Fountain), imports (Twine, Yarn, Ink)
  github/      GitHub sign-in, cloning, fetch and push, pull requests
  cli/         `easyscript` command line
app/
  src-tauri/   Tauri 2 desktop app over core
  ui/          Svelte 5 + Vite frontend
scripts/       install.sh, install.ps1: build and install from source in one command
docs/          the guide, the project file, the formats, releasing, these notes
```

## Setting up

You need Rust stable (pinned by `rust-toolchain.toml`), Node 22, and [Tauri's system dependencies](https://tauri.app/start/prerequisites/). With [mise](https://mise.jdx.dev), `mise install` sets up Rust and Node.

`./scripts/install.sh` (or `scripts\install.ps1` on Windows) builds a release and installs it, as in the README. For working on the app:

```sh
npm ci                   # frontend and Tauri CLI
npm run dev              # the app, live-reloading
npm run check            # type-check the frontend
npm test                 # frontend tests
cargo test --workspace   # Rust tests; also writes the frontend's model types
npm run build:page       # the single-file page `easyscript build` embeds
cargo run -p es-cli      # the CLI (build the page first for `build`)
```

The app's own words go through `t("English text")` (`app/ui/src/lib/i18n.svelte.ts`); translations live in `app/ui/src/lib/locales/`, and `npm test` fails when a message has none or a translation is left over.

The frontend's types in `app/ui/src/lib/bindings/` are generated from the Rust model by `cargo test` (ts-rs); commit them with the model change. To work on the read view in a browser, write a view with `easyscript build path/to/game --json -o app/ui/dev/view.json` and open the Vite dev server (`npm run dev -w app/ui`).

Tests that read a real game project take its path from `EASYSCRIPT_PROJECT` and skip when it is unset:

```sh
EASYSCRIPT_PROJECT=../na-cabeca-do-bicho cargo test --workspace
```

No team's script is committed here; public tests use fixtures written for them.

## Linux with NVIDIA

WebKitGTK's DMA-BUF renderer crashes on the NVIDIA driver under Wayland (`Error 71 (Protocol error) dispatching to Wayland display`). The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself when it finds the NVIDIA driver and the variable is unset; set it to `0` to opt out.

## The frontend against the real backend, in a browser

The `bridge` test in `app/src-tauri/src/main.rs` serves the app's commands over HTTP, so a browser (or a headless one driven by a script) can use the real backend: open a project, edit, save, commit, bring changes in. It is ignored by default and runs until stopped:

```sh
DM_BRIDGE_PORT=5299 cargo test -p es-app bridge -- --ignored --nocapture
```

`POST /invoke` takes `{"cmd": ..., "args": ...}` and answers as `invoke` would (an error with status 500); `GET /events` hands over the events emitted since the last call (`project-changed`, `clone-progress`). A page with a `window.__TAURI_INTERNALS__` whose `invoke` posts there drives the app. Two bridges on two ports, over two clones of one bare repository, are two writers on one team ([how writers use it](ux.md)).
