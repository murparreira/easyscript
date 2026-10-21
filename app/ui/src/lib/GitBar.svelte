<script lang="ts">
  // The project's git repository, in the app: the branch, what changed since the last
  // commit, and committing the project's own files. Conflicts are shown, never resolved.
  import { invoke } from "@tauri-apps/api/core";
  import type { GitStatus } from "./bindings/GitStatus";
  import type { Session } from "./bindings/Session";
  import { useEditor } from "./editor.svelte";
  import GitHubControls from "./GitHubControls.svelte";
  import { plural, t } from "./i18n.svelte";

  const editor = useEditor()!;
  let status: GitStatus | null = $state(null);
  let error: string | null = $state(null);
  let note: string | null = $state(null);

  async function refresh() {
    try {
      status = await invoke<GitStatus | null>("git_status");
    } catch (e) {
      error = String(e);
    }
  }
  // Again after every edit, save, undo and reload.
  $effect(() => {
    void editor.state;
    refresh();
  });

  let committing = $state(false);
  let message = $state("");
  let chosen: string[] = $state([]);
  const unsaved = $derived(chosen.filter((p) => editor.state.changed.includes(p)));

  function openCommit() {
    chosen = status?.files.map((f) => f.path) ?? [];
    message = "";
    error = null;
    committing = true;
  }

  async function commit() {
    try {
      const id = await invoke<string>("git_commit", { paths: chosen, message });
      committing = false;
      note = t("Committed {id}.", { id });
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  let naming = $state(false);
  let newBranch = $state("");
  async function branch(name: string, create: boolean) {
    error = null;
    try {
      editor.receive(await invoke<Session>("git_branch", { name, create }));
      naming = false;
      newBranch = "";
      note = create ? t("On the new branch {name}.", { name }) : t("On {name}.", { name });
      await refresh();
    } catch (e) {
      error = String(e);
      await refresh();
    }
  }
</script>

{#if status}
  <div class="git-bar">
    <span class="k">Git</span>
    <select aria-label={t("Branch")} value={status.branch ?? ""} onchange={(e) => branch(e.currentTarget.value, false)}>
      {#if !status.branch}<option value="">{t("(no branch)")}</option>{/if}
      {#each status.branches as b (b)}<option value={b}>{b}</option>{/each}
    </select>
    {#if naming}
      <form class="inline-form" onsubmit={(e) => { e.preventDefault(); branch(newBranch, true); }}>
        <input bind:value={newBranch} placeholder={t("new-branch-name")} aria-label={t("New branch name")} spellcheck="false" />
        <button class="btn" type="submit">{t("Create")}</button>
        <button class="btn" type="button" onclick={() => (naming = false)}>{t("Cancel")}</button>
      </form>
    {:else}
      <button type="button" class="link" onclick={() => (naming = true)}>{t("new branch")}</button>
    {/if}
    {#if status.head}<span class="tid" title={t("The last commit")}>{status.head}</span>{/if}
    {#if status.files.length > 0}
      <button class="btn" type="button" onclick={openCommit}>{plural(status.files.length, "Commit {n} file…", "Commit {n} files…")}</button>
    {:else}
      <span class="tid">{t("nothing to commit")}</span>
    {/if}
    {#if status.conflicts.length > 0}
      <span class="error">{t("Conflicts to resolve in git (not here): {files}", { files: status.conflicts.join(", ") })}</span>
    {/if}
    <GitHubControls head={status.head} />
    {#if note}<span class="tid">{note}</span>{/if}
    {#if error && !committing}<span class="error">{error}</span>{/if}
  </div>
{/if}

{#if committing && status}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && (committing = false)}>
    <div class="diff-panel" role="dialog" aria-label={t("Commit")} aria-modal="true">
    <form class="commit-form" onsubmit={(e) => { e.preventDefault(); commit(); }}>
      <div class="diff-head"><h2>{status.branch ? t("Commit to {branch}", { branch: status.branch }) : t("Commit")}</h2></div>
      <p class="lede">{t("Only the project's files you tick are committed, as they are saved on disk. Anything else in the repository is left as it is.")}</p>
      <fieldset class="files">
        {#each status.files as f (f.path)}
          <label class="tog">
            <input type="checkbox" checked={chosen.includes(f.path)} onchange={(e) => (chosen = e.currentTarget.checked ? [...chosen, f.path] : chosen.filter((p) => p !== f.path))} />
            <code>{f.path}</code> <span class="tid">{f.change === "added" ? t("added") : f.change === "deleted" ? t("deleted") : t("modified")}</span>
          </label>
        {/each}
      </fieldset>
      {#if unsaved.length > 0}<p class="warn">{t("Unsaved edits in {files}: save them first, or they are not in the commit.", { files: unsaved.join(", ") })}</p>{/if}
      <label class="le-field"><span class="k">{t("Message")}</span><textarea bind:value={message} rows="3" placeholder={t("What changed, and why")}></textarea></label>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="submit" disabled={!message.trim() || chosen.length === 0 || unsaved.length > 0}>{t("Commit")}</button>
        <button class="btn" type="button" onclick={() => (committing = false)}>{t("Cancel")}</button>
      </div>
    </form>
    </div>
  </div>
{/if}
