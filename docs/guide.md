# Using dialogue-maker

## Opening a game

Open a game project's folder (the one with `project.godot`), or **Open from GitHub** (sign in, pick one of your repositories, and the app clones it into a folder and opens it). It reads the dialogue files where they are, follows them as they change on disk, and writes only the files you change, when you save. The bar at the top switches between four views:

- **Write**: the scenes (labels) place by place, searchable; the scene as line cards, each with the speaker's face, how it is said (voice, thought, shake, face) and its words. Click a line to change it, with a preview of the balloon; add lines, choices and jumps to other scenes; move or remove them; edit `if` and `when` conditions with the project's flags at hand; add and remove scenes. Beside it, **Play** runs the script as the game would, from the scene or one you choose: lines one at a time, choices to pick, flags and variables tracked. What only the game knows (its own functions) it asks once and remembers, and a variant's question is asked by its sides' names.
- **Read**: the whole script as one document, with its documents, a flags index, who talks and the problems; searchable, filtered by the project's variants.
- **Map**: labels place by place with the jumps, choices and flags between them, where the game starts them and which nothing reaches; grids of who speaks where and which flags each place sets and reads.
- **Project**: the project file's builders, a tab each: characters, tags, flags, places, variants; or, for a project with none, a draft from the dialogue.

A game with no dialogue yet starts with its first file and scene, and **+ File** makes another. A game with no project file (`dialogue-maker.toml`) works too: the app guesses who speaks from the lines, and **Set it up** drafts a project file from the dialogue for the Project tab.

## Saving

Edits stay in the app until **Save**: the bar says what is not saved, and shows the change to each file before saving. When a file changes on disk under unsaved edits (the Godot editor saved it, the team's changes came in), the two are merged if they touch different lines; if they touch the same lines, nothing is written over: the bar says **Changed on disk: settle it**, and you keep your edits or take the version on disk, after seeing both. Undo and redo with Ctrl+Z and Ctrl+Shift+Z; save with Ctrl+S.

## Working with the team

**Work with the team** (the branch in the bar) is git, told by scene. Every few minutes the app asks `origin` what is new (a fetch: nothing of yours leaves, nothing changes; Settings turns it off). The bar shows ↓ for the team's changes your copy does not have and ↑ for your commits not sent; the scene list marks scenes changed on `origin` (↓), being changed on a teammate's branch (●), and changed since you last looked (•), and a scene says who changed it and how, before you write in it.

- **New from the team**: the commits, who made them, when, and the scenes they changed. **Bring them in** fast-forwards, or merges when you have commits of your own; your changes, committed, saved or not, are kept with the team's. Where you and the team changed the same lines, nothing changes and you are told which files.
- **Your work**: the scenes changed since the last commit; commit the project's own files (dialogue, project file, documents) as saved, and nothing else (what the team staged by hand stays staged), with a message written for you if you leave it empty. **Send** pushes the branch (never forced): on the branch the team shares, the team gets your commits the next time their app checks; on a branch of your own, with GitHub signed in, it also opens the pull request, or finds the one open.
- **Since you last looked** and **Teammates are working on**: what came in while you were away, and other branches on `origin` with the scenes they change.
- **Branch** and **GitHub**: make and switch branches; sign in to GitHub, link a repository with no `origin`.

The app checks and sends over SSH with your agent, or over HTTPS with the GitHub sign-in. **Sign in to GitHub** shows a code; GitHub, in your browser, asks you to authorize dialogue-maker and to enter it (GitHub's device flow: no password reaches the app). The app keeps the sign-in in the system's password store (Keychain, Credential Manager, Secret Service), or only until it closes when there is none; revoke it any time in GitHub's settings under Applications. A folder that is not in git has nowhere to send: open the game from GitHub instead. Conflicts git stopped on are shown, never resolved. See [how writers use it](ux.md) for how a writer starts, catches up and works beside others.

## Export and import

**Export** has the dialogue files for the game first: each file's status and a look at it, saved in place, or as a ZIP that keeps the game's folders (it unzips over the project). Then the script for others: Dialogue Manager's translation table (exporting over a table keeps its translations), voice scripts and recording sheets by character and mood, a Fountain screenplay, and JSON for other tools (a documented format). **Import** turns a Twine (`.twee`), Yarn Spinner (`.yarn`) or Ink (`.ink`) script, or a `.dialogue` file from elsewhere, into a new file in the project, after showing it and what did not convert. See [the formats](formats/README.md).

## Settings

The app's language (English or Portuguese (Brazil); it follows the system's), light or dark (or like the system), and what the script shows. Voice scripts, recording sheets and screenplays are written with their own words (headings, stage directions) in the app's language (`--language pt-BR` on the command line). The command line and import warnings are in English for now.

## The command line

```sh
dialogue-maker check path/to/game     # problems, compiler style; exit 1 on errors
dialogue-maker stats path/to/game     # labels, lines, choices, speakers, tags
dialogue-maker build path/to/game -o script.html   # the app's read view as one offline page
dialogue-maker init path/to/game      # a first dialogue-maker.toml, drafted from the dialogue
dialogue-maker export translation path/to/game -o translations.csv --translate-to pt_BR
dialogue-maker export zip path/to/game -o dialogue.zip   # the dialogue files themselves
dialogue-maker import story.ink -o path/to/game/dialogue/story.dialogue
```

All read `dialogue-maker.toml` at the game's root, or the file given with `--config`. See [the project file](project-file.md).
