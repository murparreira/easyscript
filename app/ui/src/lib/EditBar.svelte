<script lang="ts">
  // In the desktop app, at the bottom while editing: undo, redo, review and save.
  import { ask } from "@tauri-apps/plugin-dialog";
  import DiffPanel from "./DiffPanel.svelte";
  import { useEditor } from "./editor.svelte";
  import { plural, t } from "./i18n.svelte";

  const editor = useEditor()!;
  let reviewing = $state(false);
  const changed = $derived(editor.state.changed.length);

  async function discard() {
    const sure = await ask(plural(changed, "Drop the unsaved edits to {n} file? This cannot be undone.", "Drop the unsaved edits to {n} files? This cannot be undone."), {
      title: t("Discard edits"),
      kind: "warning",
    });
    if (sure) editor.discard();
  }
</script>

{#if editor.on || changed > 0}
  <div class="edit-bar gut" role="region" aria-label={t("Edits")}>
    <div class="in">
      <span class="eb-state">
        {#if changed > 0}<b>{plural(changed, "{n} file edited, not saved", "{n} files edited, not saved")}</b>{:else}{t("No unsaved edits")}{/if}
        {#if editor.busy}<span class="tid"> · {t("working…")}</span>{/if}
      </span>
      <button class="btn" type="button" onclick={editor.undo} disabled={!editor.state.can_undo || editor.busy} title="Ctrl+Z">{t("Undo")}</button>
      <button class="btn" type="button" onclick={editor.redo} disabled={!editor.state.can_redo || editor.busy} title="Ctrl+Shift+Z">{t("Redo")}</button>
      <button class="btn" type="button" onclick={() => (reviewing = true)} disabled={changed === 0}>{t("Review")}</button>
      <button class="btn primary" type="button" onclick={editor.save} disabled={changed === 0 || editor.busy} title="Ctrl+S">{t("Save")}</button>
      <button class="btn" type="button" onclick={discard} disabled={changed === 0 || editor.busy}>{t("Discard")}</button>
      {#if editor.state.conflicts.length > 0}
        <p class="error">{t("Changed on disk while you edited: {files}. Saving is refused; discard to read the new version.", { files: editor.state.conflicts.join(", ") })}</p>
      {/if}
      {#if editor.error && !editor.target}<p class="error">{editor.error}</p>{/if}
    </div>
  </div>
{/if}
{#if reviewing}<DiffPanel close={() => (reviewing = false)} />{/if}
