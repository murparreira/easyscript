<script lang="ts">
  // Sharing the work: the git branch, committing the project's own files, and on GitHub,
  // a pull request for the team to review. A folder outside git is opened from GitHub
  // instead; a repository without `origin` is linked to one of the writer's repositories.
  // Nothing reaches GitHub until the writer asks.
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { untrack } from "svelte";
  import { projects } from "./app.svelte";
  import type { GitStatus } from "./bindings/GitStatus";
  import type { PullRequest } from "./bindings/PullRequest";
  import type { RepoInfo } from "./bindings/RepoInfo";
  import type { Session } from "./bindings/Session";
  import CloneDialog from "./CloneDialog.svelte";
  import Dialog from "./Dialog.svelte";
  import { useEditor } from "./editor.svelte";
  import { github } from "./github.svelte";
  import { plural, t } from "./i18n.svelte";
  import RepoPicker from "./RepoPicker.svelte";
  import SignInDialog from "./SignInDialog.svelte";

  let { status, close, refresh }: { status: GitStatus | null; close: () => void; refresh: () => Promise<void> } = $props();
  const editor = useEditor()!;
  let error: string | null = $state(null);
  let note: string | null = $state(null);
  let busy = $state(false);

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
    }
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

  // Committing.
  let message = $state("");
  let unticked: string[] = $state([]);
  const chosen = $derived((status?.files ?? []).map((f) => f.path).filter((p) => !unticked.includes(p)));
  const unsaved = $derived(chosen.filter((p) => editor.state.changed.includes(p)));
  const commit = () =>
    run(async () => {
      const id = await invoke<string>("git_commit", { paths: chosen, message });
      message = "";
      note = t("Committed {id}.", { id });
    });

  // GitHub.
  let title = $state(untrack(() => status?.head?.replace(/^[0-9a-f]{7} /, "") ?? ""));
  let body = $state(t("Script changes made with dialogue-maker."));
  let result = $state<PullRequest | null>(null);
  const pullRequest = () =>
    run(async () => {
      result = await invoke<PullRequest>("github_pull_request", { title, body });
    });
  let linking = $state<RepoInfo | null>(null);
  const link = () =>
    run(async () => {
      if (!linking) return;
      await invoke("github_link", { owner: linking.owner, name: linking.name });
      await github.load();
      note = t("origin is now {owner}/{name} on GitHub.", { owner: linking.owner, name: linking.name });
    });
  let cloning = $state(false);
</script>

<Dialog title={t("Share")} {close} {busy} wide>
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
    <section class="share-sec">
      <h3>{t("Branch")}</h3>
      <div class="le-row">
        <select aria-label={t("Branch")} value={status.branch ?? ""} onchange={(e) => branch(e.currentTarget.value, false)} disabled={busy}>
          {#if !status.branch}<option value="">{t("(no branch)")}</option>{/if}
          {#each status.branches as b (b)}<option value={b}>{b}</option>{/each}
        </select>
        {#if naming}
          <form class="le-row" onsubmit={(e) => { e.preventDefault(); branch(newBranch, true); }}>
            <input bind:value={newBranch} placeholder={t("new-branch-name")} aria-label={t("New branch name")} spellcheck="false" />
            <button class="btn" type="submit" disabled={!newBranch.trim() || busy}>{t("Create")}</button>
            <button class="btn" type="button" onclick={() => (naming = false)}>{t("Cancel")}</button>
          </form>
        {:else}
          <button class="btn" type="button" onclick={() => (naming = true)}>{t("New branch…")}</button>
        {/if}
      </div>
      {#if status.head}<p class="hint">{t("Last commit:")} <span class="tid">{status.head}</span></p>{/if}
    </section>

    <section class="share-sec">
      <h3>{t("Changes")}</h3>
      {#if status.files.length === 0}
        <p class="hint">{t("Nothing to commit: the project's files are as in the last commit.")}</p>
      {:else}
        <p class="hint">{t("Only the project's files you tick are committed, as they are saved on disk. Anything else in the repository is left as it is.")}</p>
        <div class="files">
          {#each status.files as f (f.path)}
            <label class="tog">
              <input type="checkbox" checked={!unticked.includes(f.path)} onchange={(e) => (unticked = e.currentTarget.checked ? unticked.filter((p) => p !== f.path) : [...unticked, f.path])} />
              <code>{f.path}</code> <span class="tid">{f.change === "added" ? t("added") : f.change === "deleted" ? t("deleted") : t("modified")}</span>
            </label>
          {/each}
        </div>
        {#if unsaved.length > 0}<p class="warn">{t("Unsaved edits in {files}: save them first, or they are not in the commit.", { files: unsaved.join(", ") })}</p>{/if}
        <label class="field"><span class="k">{t("Message")}</span><textarea bind:value={message} rows="2" placeholder={t("What changed, and why")}></textarea></label>
        <div><button class="btn primary" type="button" onclick={commit} disabled={busy || !message.trim() || chosen.length === 0 || unsaved.length > 0}>{plural(chosen.length, "Commit {n} file", "Commit {n} files")}</button></div>
      {/if}
    </section>

    <section class="share-sec">
      <h3>GitHub</h3>
      {#if github.state.repo}
        <p>{t("origin is {owner}/{name} on GitHub.", { owner: github.state.repo.owner, name: github.state.repo.name })}</p>
        {#if github.state.account}
          <p class="hint">{t("Signed in as {login}.", { login: github.state.account.login })} <button type="button" class="link" onclick={() => github.signOut()}>{t("Sign out")}</button></p>
          {#if result}
            <p class="ok">{result.created ? t("Pull request #{n} opened.", { n: result.number }) : t("Pull request #{n} was already open; your commits are on it now.", { n: result.number })}</p>
            <div><button class="btn primary" type="button" onclick={() => result && openUrl(result.url)}>{t("Open it on GitHub")}</button></div>
          {:else}
            <p class="hint">{t("Pushes this branch's commits to GitHub and opens a pull request for the team to review. Nothing is forced: if GitHub has commits you do not, you are told.")}</p>
            <label class="field"><span class="k">{t("Title")}</span><input bind:value={title} /></label>
            <label class="field"><span class="k">{t("Description")}</span><textarea bind:value={body} rows="3"></textarea></label>
            <div><button class="btn primary" type="button" onclick={pullRequest} disabled={busy}>{busy ? t("Pushing…") : t("Push and open pull request")}</button></div>
          {/if}
        {:else}
          <button class="btn primary" type="button" onclick={() => github.startSignIn()}>{t("Sign in to GitHub")}</button>
        {/if}
      {:else if github.state.origin}
        <p>{t("origin is {url}, which is not on GitHub: push and open pull requests with git.", { url: github.state.origin })}</p>
      {:else}
        <p>{t("This repository has no origin yet: link it to one of your repositories on GitHub to push and open pull requests.")}</p>
        {#if github.state.account}
          <RepoPicker bind:chosen={linking} />
          <div><button class="btn primary" type="button" onclick={link} disabled={!linking || busy}>{t("Link to this repository")}</button></div>
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
