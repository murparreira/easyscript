<script lang="ts">
  // The unsaved edits as they will change the files; save them, or drop them.
  import { ask } from "@tauri-apps/plugin-dialog";
  import type { FileDiff } from "./bindings/FileDiff";
  import Dialog from "./Dialog.svelte";
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

  async function discard() {
    const n = editor.state.changed.length;
    const sure = await ask(plural(n, "Drop the unsaved edits to {n} file? This cannot be undone.", "Drop the unsaved edits to {n} files? This cannot be undone."), {
      title: t("Discard edits"),
      kind: "warning",
    });
    if (sure && (await editor.discard())) close();
  }
</script>

<Dialog title={t("Unsaved changes")} {close} busy={editor.busy} wide>
  {#if editor.state.conflicts.length > 0}
    <p class="error">{t("Changed on disk while you edited: {files}. Saving is refused; discard to read the new version.", { files: editor.state.conflicts.join(", ") })}</p>
  {/if}
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
  <p class="hint">{t("Only these files are written; a file changed on disk since it was read is not.")}</p>

  {#snippet actions()}
    <button class="btn primary" type="button" onclick={save} disabled={editor.busy || !diffs?.length}>{plural(diffs?.length ?? 0, "Save {n} file", "Save {n} files")}</button>
    <button class="btn danger" type="button" onclick={discard} disabled={editor.busy || editor.state.changed.length === 0}>{t("Discard…")}</button>
  {/snippet}
</Dialog>
