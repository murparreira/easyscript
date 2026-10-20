# dialogue-maker

Desktop workbench for game scripts: Rust core + Tauri 2 app. It reads a game project's dialogue files in place, shows the script, edits it back losslessly, opens PRs. Plan, decisions and milestones: `PLAN.md` (read it first).

Reference project: *Na Cabeça do Bicho*, at `../na-cabeca-do-bicho` (add it with `--add-dir`). Its `tools/gerar_documentos/` is the Python tool this replaces. Read it; do not change that repo from here unless asked.

## Hard rules

1. Lossless: `print(parse(file)) == file` for every file. Unknown syntax is an opaque node kept verbatim.
2. Never write a file the user did not change, or over one that changed on disk since it was read.
3. No game-specific code in the core: per-project knowledge goes in the project file, engine or game readers behind an adapter trait.
4. No team's script committed here. Real-project tests read `DIALOGUE_MAKER_PROJECT` and skip when unset.
5. Offline; no telemetry.

## Conventions

- Chat with the user in English. The reference game's text and ids are pt-BR.
- No `Co-Authored-By` Claude trailers in commits, no "Generated with Claude Code" footer in PRs.
- Commit only when asked.
