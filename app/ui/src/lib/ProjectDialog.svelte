<script lang="ts">
  // The project open, and opening another: a folder, a repository from GitHub, a recent one.
  import { projects } from "./app.svelte";
  import CloneDialog from "./CloneDialog.svelte";
  import Dialog from "./Dialog.svelte";
  import { t } from "./i18n.svelte";

  let { close, path, openOther }: { close: () => void; path: string | null; openOther: () => void } = $props();
  let cloning = $state(false);

  const recent: string[] = (() => {
    try {
      return (JSON.parse(localStorage.getItem("dialogue-maker:recent-projects") ?? "[]") as string[]).filter((p) => p !== path);
    } catch {
      return [];
    }
  })();

  function open(folder: string) {
    close();
    projects.open(folder);
  }
</script>

<Dialog title={t("Project")} {close}>
  {#if path}
    <div class="field">
      <span class="k">{t("Open now")}</span>
      <code class="path">{path}</code>
      <p class="hint">{t("The app reads the dialogue files where they are, follows changes on disk, and writes only the files you change, when you save.")}</p>
    </div>
  {/if}
  {#if recent.length > 0}
    <div class="recent">
      <div class="k">{t("Recent")}</div>
      {#each recent as folder (folder)}
        <button type="button" class="recent-item" onclick={() => open(folder)}>
          <span class="name">{folder.split(/[\\/]/).pop()}</span>
          <span class="tid">{folder}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#snippet actions()}
    <button class="btn primary" type="button" onclick={() => { close(); openOther(); }}>{t("Open a folder…")}</button>
    <button class="btn" type="button" onclick={() => (cloning = true)}>{t("Open from GitHub…")}</button>
  {/snippet}
</Dialog>
{#if cloning}<CloneDialog close={() => (cloning = false)} opened={open} />{/if}
