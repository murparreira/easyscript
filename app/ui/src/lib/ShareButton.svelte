<script lang="ts">
  // In the bar: the branch, and whether there is something to commit. Opens the share
  // panel (git and GitHub).
  import { invoke } from "@tauri-apps/api/core";
  import type { GitStatus } from "./bindings/GitStatus";
  import { useEditor } from "./editor.svelte";
  import { github } from "./github.svelte";
  import { plural, t } from "./i18n.svelte";
  import SharePanel from "./SharePanel.svelte";

  const editor = useEditor()!;
  let status = $state<GitStatus | null>(null);
  let open = $state(false);

  async function refresh() {
    try {
      status = await invoke<GitStatus | null>("git_status");
    } catch {
      status = null;
    }
    await github.load();
  }
  // Again after every edit, save, undo and reload.
  $effect(() => {
    void editor.state;
    refresh();
  });
  const uncommitted = $derived(status?.files.length ?? 0);
</script>

<button
  class="btn share"
  type="button"
  onclick={() => (open = true)}
  title={status ? plural(uncommitted, "{n} file changed since the last commit", "{n} files changed since the last commit") : t("Git and GitHub")}
>
  {#if status}
    <svg class="glyph" viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Zm0 9a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Zm9-6a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM3.5 5v6M12.5 8c0 2-2 2.5-4 2.5S5 11 4.2 11.8" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
    <span class="branch-name">{status.branch ?? t("(no branch)")}</span>
    {#if uncommitted > 0}<span class="badge">{uncommitted}</span>{/if}
  {:else}
    {t("Share")}
  {/if}
</button>

{#if open}<SharePanel {status} close={() => (open = false)} {refresh} />{/if}
