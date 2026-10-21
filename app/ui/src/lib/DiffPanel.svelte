<script lang="ts">
  // The unsaved edits as they will change the files.
  import type { FileDiff } from "./bindings/FileDiff";
  import { useEditor } from "./editor.svelte";
  import { plural, t } from "./i18n.svelte";

  let { close }: { close: () => void } = $props();
  const editor = useEditor()!;
  let diffs: FileDiff[] | null = $state(null);
  $effect(() => {
    // Again whenever the edits change.
    void editor.state;
    editor.diffs().then((d) => (diffs = d));
  });

  async function save() {
    if (await editor.save()) close();
  }
</script>

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
  <div class="diff-panel" role="dialog" aria-label={t("Unsaved changes")} aria-modal="true">
    <div class="diff-head">
      <h2>{t("Unsaved changes")}</h2>
      <button class="btn" type="button" onclick={close}>{t("Close")}</button>
    </div>
    {#if diffs === null}
      <p class="nothing">{t("Reading…")}</p>
    {:else if diffs.length === 0}
      <p class="nothing">{t("Nothing to save.")}</p>
    {:else}
      {#each diffs as file (file.path)}
        <section class="diff-file">
          <h3><code>{file.path}</code>{#if editor.state.conflicts.includes(file.path)} <span class="error">{t("changed on disk")}</span>{/if}</h3>
          {#each file.hunks as hunk, i (i)}
            <pre class="hunk">{#each hunk as line, j (j)}<span class="dl {line.kind}">{line.kind === "added" ? "+" : line.kind === "removed" ? "-" : " "} {line.text}
</span>{/each}</pre>
          {/each}
        </section>
      {/each}
    {/if}
    {#if editor.error}<p class="error">{editor.error}</p>{/if}
    <div class="le-actions">
      <button class="btn primary" type="button" onclick={save} disabled={editor.busy || !diffs?.length}>{plural(diffs?.length ?? 0, "Save {n} file", "Save {n} files")}</button>
      <span class="tid">{t("Only these files are written; a file changed on disk since it was read is not.")}</span>
    </div>
  </div>
</div>
