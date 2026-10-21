<img src="app/icon.svg" width="88" alt="">

# easyscript

Write a game's dialogue where it lives. easyscript opens a Godot game's [Dialogue Manager](https://github.com/nathanhoad/godot_dialogue_manager) files in place, shows the whole script, lets writers write it and play it, and shares the work with the team through git and GitHub. For Mac, Windows and Linux; it works offline.

The first format is Dialogue Manager 3 `.dialogue` files (Godot 4).

## What it does

- **Write** scenes as cards, one per line: who speaks, their face and voice, and the words. Add choices, conditions, flags and jumps to other scenes without touching the syntax. Only the lines you change are written, and only when you save; comments, blank lines and indentation stay as they were.
- **Play** the scene beside it, as the game would: lines one at a time, choices to pick, flags tracked. What only the game knows, it asks once.
- **Read** the whole script as one document, searchable and filtered by the game's modes. The **Map** shows how scenes lead to each other, who speaks where, and which flags each place sets and reads.
- **Work with the team** through git. It shows, scene by scene, what teammates changed and are changing. It brings their changes in without losing yours, commits and sends your work, and opens the pull request.
- **Export**:
  - the dialogue files for the game;
  - a translation table;
  - voice scripts by character and mood;
  - a Fountain screenplay;
  - JSON for other tools.
- **Import** Twine, Yarn Spinner and Ink scripts.
- **Command line**: check the script, count it, and write it as one HTML page to share.
- In English and Portuguese (Brazil).

## Install

One command, with nothing to install first. It puts the latest easyscript and its command line on this computer, for you alone (no administrator). Run it again to update.

**macOS and Linux**, in a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/murparreira/easyscript/main/scripts/install.sh | sh
```

**Windows**, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/murparreira/easyscript/main/scripts/install.ps1 | iex
```

The app lands in Applications on macOS, in the applications menu on Linux, and in the Start menu on Windows. It runs on macOS (Apple silicon and Intel), Windows 10 and 11, and Linux on x86_64.

### Or download an installer

The [releases page](https://github.com/murparreira/easyscript/releases) has an installer for each system, and the command line on its own:

| System | File | The first time |
|---|---|---|
| Windows 10 and 11 | `…-setup.exe` or `.msi` | SmartScreen warns about an unknown app: **More info → Run anyway** |
| macOS | `.dmg` | macOS refuses to open it: **System Settings → Privacy & Security → Open Anyway** |
| Linux, any distribution | `.AppImage` | Make it executable (`chmod +x`) and run it |
| Debian, Ubuntu | `.deb` | `sudo apt install ./easyscript_*.deb` |
| Fedora, openSUSE | `.rpm` | `sudo dnf install ./easyscript-*.rpm` |

Opened from a browser download, the installers warn because they are not signed yet ([releasing](docs/release.md) says what signing needs). Installed with the command above, they do not ask.

To build it yourself from this repository instead, see [developing](docs/development.md).

## Start

1. Open the game's folder (the one with `project.godot`). Or choose **Open from GitHub**: the app clones the team's repository into a folder and opens it there.
2. Pick a scene on the left and click a line to change it. **▶ Play from here** plays it.
3. Save with **Save** or Ctrl+S. Then open **Work with the team** (the branch button in the bar) to commit and send your work.

A game with no dialogue yet starts with its first file. A game with no project file gets one drafted from its dialogue: choose **Set it up**.

## Documentation

- [Using the app](docs/guide.md): the views, saving, working with the team, export and import, settings, the command line.
- [The project file](docs/project-file.md), `easyscript.toml`: characters, tags, places, flags, variants.
- [The formats](docs/formats/README.md) it exports and imports.
- [How writers use it](docs/ux.md): starting, catching up, and working beside others.
- [Developing](docs/development.md), [releasing](docs/release.md), and [the plan](PLAN.md).

## Licence

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
