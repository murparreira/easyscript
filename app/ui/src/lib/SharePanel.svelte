<script lang="ts">
  // Working with the team through git: what came in from the team (brought in with a
  // click, the writer's own changes kept), the writer's work (save, commit, send, and on
  // GitHub a pull request), what changed since they last looked, and what teammates are
  // changing on their own branches; all told by scene. A folder outside git is opened
  // from GitHub instead; a repository without `origin` is linked to one on GitHub.
  // Nothing leaves the machine until the writer sends it.
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { untrack } from "svelte";
  import { projects } from "./app.svelte";
  import type { GitStatus } from "./bindings/GitStatus";
  import type { RepoInfo } from "./bindings/RepoInfo";
  import type { SceneRef } from "./bindings/SceneRef";
  import type { Session } from "./bindings/Session";
  import type { Updated } from "./bindings/Updated";
  import CloneDialog from "./CloneDialog.svelte";
  import CommitList from "./CommitList.svelte";
  import Dialog from "./Dialog.svelte";
  import { useEditor } from "./editor.svelte";
  import { github } from "./github.svelte";
  import { plural, t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";
  import RepoPicker from "./RepoPicker.svelte";
  import SignInDialog from "./SignInDialog.svelte";
  import { type Sent, ago, sceneName, useTeam } from "./team.svelte";

  let { status, close, refresh }: { status: GitStatus | null; close: () => void; refresh: () => Promise<void> } = $props();
  const editor = useEditor()!;
  const reader = useReader();
  const team = useTeam();
  let error: string | null = $state(null);
  let note: string | null = $state(null);
  let busy = $state(false);
  const working = $derived(busy || editor.busy || (team?.busy ?? false));
  const standing = $derived(team?.state ?? null);

  async function run(work: () => Promise<void>) {
    busy = true;
    error = null;
    note = null;
    try {
      await work();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      await refresh();
      await team?.load();
    }
  }

  /** Open a scene to write, and get out of the way. */
  function openScene(scene: SceneRef) {
    const file = reader.view.script.files.find((f) => f.path === scene.path);
    const label = scene.label ?? file?.sections.flatMap((s) => s.labels)[0]?.id;
    if (label) {
      reader.scene = label;
      reader.tab = "write";
      close();
    }
  }

  // From the team.
  let brought: Updated | null = $state(null);
  async function bringIn() {
    error = null;
    note = null;
    brought = (await team?.bringIn()) ?? null;
    if (!brought) error = editor.error;
    await refresh();
  }

  // The writer's work.
  let message = $state("");
  let unticked: string[] = $state([]);
  const chosen = $derived((status?.files ?? []).map((f) => f.path).filter((p) => !unticked.includes(p)));
  const unsaved = $derived(chosen.filter((p) => editor.state.changed.includes(p)));
  const touched = $derived((standing?.uncommitted ?? []).filter((s) => chosen.includes(s.path)));
  const auto = $derived(t("Change {scenes}", { scenes: [...new Set(touched.map(sceneName))].slice(0, 4).join(", ") || chosen.join(", ") }));
  const commit = () =>
    run(async () => {
      const id = await invoke<string>("git_commit", { paths: chosen, message: message.trim() || auto });
      message = "";
      note = t("Committed {id}.", { id });
    });

  const onShared = $derived(!!standing?.shared && standing.branch === standing.shared);
  const toSend = $derived(standing ? standing.outgoing.length > 0 || (!standing.published && !!standing.branch && !onShared) : false);
  const willOpenPr = $derived(!onShared && !!github.state.repo && !!github.state.account);
  let title = $state(untrack(() => team?.state?.outgoing[0]?.summary ?? status?.head?.replace(/^[0-9a-f]{7} /, "") ?? ""));
  let body = $state(t("Script changes made with easyscript."));
  let sent: Sent | null = $state(null);
  async function send() {
    error = null;
    note = null;
    sent = await (team?.send(title, body) ?? null);
    if (!sent) error = editor.error;
    await refresh();
  }

  // Branches.
  let naming = $state(false);
  let newBranch = $state("");
  const branch = (name: string, create: boolean) =>
    run(async () => {
      editor.receive(await invoke<Session>("git_branch", { name, create }));
      naming = false;
      newBranch = "";
      note = create ? t("On the new branch {name}.", { name }) : t("On {name}.", { name });
    });

  // GitHub.
  let linking = $state<RepoInfo | null>(null);
  const link = () =>
    run(async () => {
      if (!linking) return;
      await invoke("github_link", { owner: linking.owner, name: linking.name });
      await github.load();
      note = t("origin is now {owner}/{name} on GitHub.", { owner: linking.owner, name: linking.name });
    });
  let cloning = $state(false);
  const since = $derived((standing?.since ?? []).filter((c) => !c.mine));
</script>

<Dialog title={t("Work with the team")} {close} busy={working} wide>
  {#if !status}
    <section class="share-sec">
      <h3>{t("This folder is not in git")}</h3>
      <p>{t("Saving writes your edits to the files in this folder, and that is all. To share them through GitHub, as a branch and a pull request for the team to review, open the game from GitHub: the app clones its repository into a folder and opens it there.")}</p>
      <button class="btn primary" type="button" onclick={() => (cloning = true)}>{t("Open from GitHub…")}</button>
    </section>
  {:else}
    {#if status.conflicts.length > 0}
      <p class="error">{t("Conflicts to resolve in git (not here): {files}", { files: status.conflicts.join(", ") })}</p>
    {/if}
    {#if team && github.state.origin}
      <p class="team-line">
        {#if team.checking}{t("Asking origin what is new…")}
        {:else if team.unreachable}<span class="warn">{t("origin could not be reached: {why}", { why: team.unreachable })}</span>
        {:else if team.checked}{t("Checked with origin {when}.", { when: ago(team.checked / 1000) })}
        {:else}{t("Not checked with origin yet.")}{/if}
        <button type="button" class="link" onclick={() => team.check()} disabled={team.checking}>{t("Check now")}</button>
      </p>
    {/if}

    {#if standing && standing.incoming.length > 0}
      <section class="share-sec news">
        <h3>{t("New from the team")}</h3>
        <p>{plural(standing.incoming.length, "{n} change on origin that your copy does not have yet.", "{n} changes on origin that your copy does not have yet.")}</p>
        <CommitList commits={standing.incoming} open={openScene} />
        <p class="hint">{t("Bringing them in keeps your own changes, saved or not. Where you and the team changed the same lines, nothing changes and you are told.")}</p>
        <button class="btn primary" type="button" onclick={bringIn} disabled={working}>{team?.busy ? t("Bringing in…") : t("Bring them in")}</button>
      </section>
    {/if}
    {#if brought}
      {#if brought.conflicts.length > 0}
        <p class="error">{t("You and the team changed the same lines in {files}, so nothing was brought in. Commit your changes and settle it in git, or ask a teammate.", { files: brought.conflicts.join(", ") })}</p>
      {:else if brought.commits > 0}
        <div class="ok">
          {plural(brought.commits, "Brought in {n} change.", "Brought in {n} changes.")}
          {#if brought.merged.length > 0}{t("Your changes to {files} were kept with the team's.", { files: brought.merged.join(", ") })}{/if}
        </div>
      {/if}
    {/if}

    <section class="share-sec">
      <h3>{t("Your work")}</h3>
      {#if editor.state.changed.length > 0}
        <div class="le-row">
          <p class="warn">{plural(editor.state.changed.length, "{n} file not saved: only what is saved can be committed.", "{n} files not saved: only what is saved can be committed.")}</p>
          <button class="btn" type="button" onclick={() => editor.save()} disabled={working}>{t("Save")}</button>
        </div>
      {/if}
      {#if status.files.length === 0}
        {#if editor.state.changed.length === 0}<p class="hint">{t("Nothing to commit: the project's files are as in the last commit.")}</p>{/if}
      {:else}
        {#if touched.length > 0}
          <div class="scene-chips">
            <span class="k">{t("Changed")}</span>
            {#each touched as s (s.path + "#" + s.label)}<button type="button" class="chip scene-chip" onclick={() => openScene(s)} title={s.path}>{sceneName(s)}</button>{/each}
          </div>
        {/if}
        <details class="files-details">
          <summary>{plural(chosen.length, "{n} file to commit", "{n} files to commit")}</summary>
          <p class="hint">{t("Only the project's files you tick are committed, as they are saved on disk. Anything else in the repository is left as it is.")}</p>
          <div class="files">
            {#each status.files as f (f.path)}
              <label class="tog">
                <input type="checkbox" checked={!unticked.includes(f.path)} onchange={(e) => (unticked = e.currentTarget.checked ? unticked.filter((p) => p !== f.path) : [...unticked, f.path])} />
                <code>{f.path}</code> <span class="tid">{f.change === "added" ? t("added") : f.change === "deleted" ? t("deleted") : t("modified")}</span>
              </label>
            {/each}
          </div>
        </details>
        <label class="field"><span class="k">{t("What changed, and why")}</span><textarea bind:value={message} rows="2" placeholder={auto}></textarea></label>
        {#if onShared}<p class="hint">{t("You are on {branch}, the branch the whole team shares: once sent, these commits reach everyone. To have them reviewed first, make a branch of your own before committing.", { branch: standing?.branch ?? "" })}</p>{/if}
        <div><button class="btn primary" type="button" onclick={commit} disabled={working || chosen.length === 0 || unsaved.length > 0}>{plural(chosen.length, "Commit {n} file", "Commit {n} files")}</button></div>
      {/if}

      {#if standing && toSend}
        <div class="send">
          <div class="k">{plural(standing.outgoing.length, "{n} commit to send", "{n} commits to send")}</div>
          <CommitList commits={standing.outgoing} open={openScene} limit={3} />
          {#if willOpenPr}
            <label class="field"><span class="k">{t("Pull request title")}</span><input bind:value={title} /></label>
            <label class="field"><span class="k">{t("Description")}</span><textarea bind:value={body} rows="2"></textarea></label>
          {/if}
          <p class="hint">
            {#if onShared}{t("Sends your commits to {branch} on origin: the team gets them the next time their app checks.", { branch: standing.branch ?? "" })}
            {:else if willOpenPr}{t("Sends this branch to origin and opens a pull request for the team to review.")}
            {:else if github.state.repo}{t("Sends this branch to origin. Sign in to GitHub to open a pull request too.")}
            {:else}{t("Sends this branch to origin.")}{/if}
          </p>
          <div class="le-row">
            <button class="btn primary" type="button" onclick={send} disabled={working || (onShared && standing.incoming.length > 0)}>{team?.busy ? t("Sending…") : willOpenPr ? t("Send and open pull request") : t("Send")}</button>
            {#if onShared && standing.incoming.length > 0}<span class="hint">{t("Bring the team's changes in first.")}</span>{/if}
          </div>
        </div>
      {/if}
      {#if sent}
        {#if sent.pull_request}
          <p class="ok">{sent.pull_request.created ? t("Pull request #{n} opened.", { n: sent.pull_request.number }) : t("Pull request #{n} was already open; your commits are on it now.", { n: sent.pull_request.number })}
            <button type="button" class="link" onclick={() => sent?.pull_request && openUrl(sent.pull_request.url)}>{t("Open it on GitHub")}</button></p>
        {:else}
          <p class="ok">{t("Sent {branch} to origin.", { branch: sent.branch })}</p>
        {/if}
      {/if}
    </section>

    {#if since.length > 0}
      <section class="share-sec">
        <h3>{t("Since you last looked")}</h3>
        <CommitList commits={since} open={openScene} limit={5} />
        <div><button class="btn" type="button" onclick={() => team?.markSeen()}>{t("Mark all as seen")}</button></div>
      </section>
    {/if}

    {#if standing && standing.others.length > 0}
      <section class="share-sec">
        <h3>{t("Teammates are working on")}</h3>
        <p class="hint">{t("Branches on origin with changes not on {branch} yet. Changing the same scenes at the same time means sorting it out later: talk first.", { branch: standing.shared ?? "" })}</p>
        <ul class="commits">
          {#each standing.others as work (work.branch)}
            <li>
              <div class="commit-head">
                <span class="who">{work.last.mine ? t("You") : work.last.author}</span>
                <span class="when">{ago(work.last.time)}</span>
                <code class="tid">{work.branch}</code>
                <span class="tid">{plural(work.ahead, "{n} commit", "{n} commits")}</span>
              </div>
              <div class="summary">{work.last.summary}</div>
              <div class="scene-chips">
                {#each work.last.scenes as s (s.path + "#" + s.label)}<button type="button" class="chip scene-chip" onclick={() => openScene(s)} title={s.path}>{sceneName(s)}</button>{/each}
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="share-sec">
      <h3>{t("Branch")}</h3>
      <div class="le-row">
        <select aria-label={t("Branch")} value={status.branch ?? ""} onchange={(e) => branch(e.currentTarget.value, false)} disabled={working}>
          {#if !status.branch}<option value="">{t("(no branch)")}</option>{/if}
          {#each status.branches as b (b)}<option value={b}>{b}</option>{/each}
        </select>
        {#if naming}
          <form class="le-row" onsubmit={(e) => { e.preventDefault(); branch(newBranch, true); }}>
            <input bind:value={newBranch} placeholder={t("new-branch-name")} aria-label={t("New branch name")} spellcheck="false" />
            <button class="btn" type="submit" disabled={!newBranch.trim() || working}>{t("Create")}</button>
            <button class="btn" type="button" onclick={() => (naming = false)}>{t("Cancel")}</button>
          </form>
        {:else}
          <button class="btn" type="button" onclick={() => (naming = true)}>{t("New branch…")}</button>
        {/if}
      </div>
      {#if status.head}<p class="hint">{t("Last commit:")} <span class="tid">{status.head}</span></p>{/if}
    </section>

    <section class="share-sec">
      <h3>GitHub</h3>
      {#if github.state.repo}
        <p>{t("origin is {owner}/{name} on GitHub.", { owner: github.state.repo.owner, name: github.state.repo.name })}</p>
        {#if github.state.account}
          <p class="hint">{t("Signed in as {login}.", { login: github.state.account.login })} <button type="button" class="link" onclick={() => github.signOut()}>{t("Sign out")}</button></p>
        {:else}
          <p class="hint">{t("Signing in lets the app reach a private repository and open pull requests.")}</p>
          <button class="btn" type="button" onclick={() => github.startSignIn()}>{t("Sign in to GitHub")}</button>
        {/if}
      {:else if github.state.origin}
        <p>{t("origin is {url}. The app checks it and sends to it with your git credentials; pull requests need a repository on GitHub.", { url: github.state.origin })}</p>
      {:else}
        <p>{t("This repository has no origin yet: link it to one of your repositories on GitHub to send your work and open pull requests.")}</p>
        {#if github.state.account}
          <RepoPicker bind:chosen={linking} />
          <div><button class="btn primary" type="button" onclick={link} disabled={!linking || working}>{t("Link to this repository")}</button></div>
        {:else}
          <button class="btn primary" type="button" onclick={() => github.startSignIn()}>{t("Sign in to GitHub")}</button>
        {/if}
      {/if}
    </section>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if note}<p class="ok">{note}</p>{/if}
</Dialog>
<SignInDialog />
{#if cloning}<CloneDialog close={() => (cloning = false)} opened={(path) => { close(); projects.open(path); }} />{/if}
