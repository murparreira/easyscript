# How writers use dialogue-maker

Notes from using the app as three people: a writer starting a game's dialogue, a writer coming back to a game the team has been changing, and two writers working on one script at the same time. Each part says what was wrong, what the app does now, and what is still open.

The principles behind the answers:

- **The script is the game's files.** The app adds no database and no server. Git, through the repository's `origin`, is what the team shares.
- **Tell it by scene.** A writer thinks "Beto changed `taxista_confissao`", not "3 hunks in `avenida_vereador_barata.dialogue`". Commits, branches and conflicts are named by the scenes (labels) they touch.
- **Never lose a writer's words.** Changes from outside are merged into unsaved work when they touch other lines. When they touch the same lines, nothing is overwritten and the writer chooses.
- **Nothing leaves without asking.** Checking `origin` is a fetch; sending is a button.

## Starting

### A new game, no dialogue yet

**Was:** the Write view said "No scenes in this project yet", and **+ Scene** was disabled because there was no scene to add after. That was a dead end; the only way out was making a file by hand.

**Now:** an empty project opens on **No dialogue here yet**. It says in two sentences what a dialogue file and a scene are, and asks for a file name and a first scene. The file is suggested in `dialogue/`, Dialogue Manager's habit, or wherever the project's other dialogue files are. The new file is an unsaved edit like any other: it can be undone, it shows in the diff, and it is written on save. **+ File** beside **+ Scene** makes more files. The Play panel plays the first scene written.

### Joining a game that has dialogue, with no project file

This is *Na Cabeça do Bicho* today, and how the reference game first looked.

**Was:**

- The scene list named each file by its full path (`src/gameplay/dialogues/alameda_d…`), cut off, under "Not in a place".
- Naming a new scene spilled the list over the scene: the hint held an unbreakable path, and the list's grid column grew past its 250 px.
- Setting up a project file was hidden in the Project tab.

**Now:**

- Files are named by their file name and grouped by folder. Configured places still win.
- The list stays in its column.
- A card at the top of the scene list says what the app is guessing (who speaks, where scenes play) and offers **Set it up**, which drafts the project file and opens the Project tab. **Not now** hides the card for that project.

**Open:**

- A first-run walk-through: three steps, scene list → line → Play.
- Faces drafted from a reactions folder, so a project without a project file still shows portraits.

## Catching up

A writer back after a day or a week, on a script the team kept changing.

**Was:** nothing in the app said what changed. The writer had to read `git log` outside the app, by file and line.

**Now:**

- The app remembers, per project, the last commit the writer saw.
- **Since you last looked** (in the team panel) lists the commits that came in since: who made each, when, its message, and the scenes it changed. A click opens a scene.
- In the scene list those scenes carry a •, and a card counts them, with **Show them** (the list filters to them) and **Got it**.
- Opening one of those scenes shows who changed it and their message. **Got it** there clears that scene alone; it comes back if someone changes it again.
- The writer's own commits never count as news.

**Open:**

- A scene's history: who wrote each line (git blame by scene), and earlier versions to compare.
- What a commit did to a scene, as lines before and after, rather than just its name.

## Working beside others

"How would one writer's change reach another writer who is changing the script right now?"

Without a server (rule 5), the answer is git, made quick and told by scene:

| Situation | What the writer sees | What happens |
|---|---|---|
| A teammate sends a change to the shared branch | Within 3 minutes (sooner when the window comes back), ↓1 on the branch button and ↓ on the scene in the list | Nothing changes until **Bring them in** |
| …to the scene being written | A banner on the scene: "Ana changed this scene on origin… bring it in before you write here" | The writer brings it in first and writes on the latest |
| …to other lines of a file with unsaved edits | — | Bringing it in merges: the team's change and the unsaved edits are both kept, and only the edits stay unsaved |
| …to the same lines as unsaved edits | **Changed on disk: settle it**, with their change and the writer's edits side by side | **Keep my edits** (save writes them over theirs, seen) or **Take theirs** |
| …to the same lines as committed or saved work | "You and the team changed the same lines in …" | Nothing changes at all; the files are named |
| A teammate works on a branch of their own | ● on the scene; a banner on it: "Beto is also changing this scene, on the branch beto-taxi (20 minutes ago)… talk before you both change the same lines" | — |
| The Godot editor, or a synced folder (Syncthing, Dropbox), changes a file | The scene updates within a second | Same merge as above; an open line editor follows its line, or closes if the line is gone |
| The writer sends | ↑ shows commits not sent | On the shared branch: pushed, and teammates see it on their next check. On a branch of one's own: pushed, and its pull request opened |

Each row was tried with the real backend and two clones of one repository ("Ana" and "Beto"): see Checking below.

**Open:**

- **Work in progress is invisible until sent.** A writer's unsaved or unsent change cannot warn a teammate. One answer is a personal draft branch pushed in the background (opt-in), which the ● markers would then show. It needs a decision on noise in the repository.
- **Same-line conflicts in git are not resolved in the app.** It stops and names the files. Next step: per scene, mine or theirs, the way unsaved edits are settled now.
- **Neighbouring lines count as the same place**, as in git. Two writers changing consecutive lines of one scene get a conflict. A merge that knows dialogue lines could take both.
- **Real-time co-editing** (both cursors in one scene) needs a server and is out of scope for an offline app.
- **Commit and send in one step**, with a branch named for the writer when they are on the shared branch and the team wants reviews.

## Everyday writing

**Fixed in this pass:**

- The scene list spilled over the scene when naming a new scene.
- The Play panel stayed empty after the first scene of a new project.
- Diffs scrolled sideways on long lines; they wrap now.

**Open, smaller:**

- Line tools appear on hover only; they could show on the selected line.
- Problems (a jump to a missing scene) could be shown on the scene in Write, not only in Read.
- Raw tags (`reaction=…`) show as chips when the project file does not explain them. With **Ids** off they hide, but the default is on.

## Checking

The frontend runs in a browser against the real backend: the app's commands over HTTP, from the `bridge` test in `app/src-tauri/src/main.rs`. It is ignored by default and runs until stopped:

```sh
DM_BRIDGE_PORT=5299 cargo test -p dm-app bridge -- --ignored --nocapture
```

A page with a fake `__TAURI_INTERNALS__` forwards `invoke` to it and polls `GET /events` for `project-changed`. Two bridges on two ports, over two clones of one bare repository, are two writers on one team. The team logic itself is tested in `crates/core/tests/team.rs` and `crates/core/tests/workspace.rs`.
