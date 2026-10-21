<script lang="ts">
  // In the bar: the branch, what came in from the team that is not here yet (↓), what is
  // here that the team does not have (↑), and files changed since the last commit. Opens
  // the panel to work with the team.
  import { invoke } from "@tauri-apps/api/core";
  import type { GitStatus } from "./bindings/GitStatus";
  import { useEditor } from "./editor.svelte";
  import { github } from "./github.svelte";
  import { plural, t } from "./i18n.svelte";
  import SharePanel from "./SharePanel.svelte";
  import { useTeam } from "./team.svelte";

  const editor = useEditor()!;
  const team = useTeam();
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
  const incoming = $derived(team?.state?.incoming.length ?? 0);
  const outgoing = $derived(team?.state?.outgoing.length ?? 0);
  const title = $derived(
    status
      ? [
          incoming > 0 ? plural(incoming, "{n} change from the team to bring in", "{n} changes from the team to bring in") : null,
          outgoing > 0 ? plural(outgoing, "{n} commit to send", "{n} commits to send") : null,
          uncommitted > 0 ? plural(uncommitted, "{n} file changed since the last commit", "{n} files changed since the last commit") : null,
          team?.unreachable ? t("origin could not be reached") : null,
        ]
          .filter(Boolean)
          .join(" · ") || t("Up to date with the team")
      : t("Git and GitHub"),
  );
</script>

<button class="btn share" class:news={incoming > 0} type="button" onclick={() => (open = true)} {title}>
  {#if status}
    <svg class="glyph" viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Zm0 9a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Zm9-6a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM3.5 5v6M12.5 8c0 2-2 2.5-4 2.5S5 11 4.2 11.8" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
    <span class="branch-name">{status.branch ?? t("(no branch)")}</span>
    {#if incoming > 0}<span class="badge in" aria-label={plural(incoming, "{n} change from the team to bring in", "{n} changes from the team to bring in")}>↓{incoming}</span>{/if}
    {#if outgoing > 0}<span class="badge out" aria-label={plural(outgoing, "{n} commit to send", "{n} commits to send")}>↑{outgoing}</span>{/if}
    {#if uncommitted > 0}<span class="badge" aria-label={plural(uncommitted, "{n} file changed since the last commit", "{n} files changed since the last commit")}>{uncommitted}</span>{/if}
    {#if team?.checking}<span class="spin" aria-hidden="true"></span>{/if}
  {:else}
    {t("Share")}
  {/if}
</button>

{#if open}<SharePanel {status} close={() => (open = false)} {refresh} />{/if}
