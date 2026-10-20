<script lang="ts">
  // In the desktop app, at the bottom while editing: undo, redo, review and save.
  import { ask } from "@tauri-apps/plugin-dialog";
  import DiffPanel from "./DiffPanel.svelte";
  import { useEditor } from "./editor.svelte";

  const editor = useEditor()!;
  let reviewing = $state(false);
  const changed = $derived(editor.state.changed.length);

  async function discard() {
    const sure = await ask(`Drop the unsaved edits to ${changed} ${changed === 1 ? "file" : "files"}? This cannot be undone.`, {
      title: "Discard edits",
      kind: "warning",
    });
    if (sure) editor.discard();
  }
</script>

{#if editor.on || changed > 0}
  <div class="edit-bar gut" role="region" aria-label="Edits">
    <div class="in">
      <span class="eb-state">
        {#if changed > 0}<b>{changed} {changed === 1 ? "file" : "files"} edited</b>, not saved{:else}No unsaved edits{/if}
        {#if editor.busy}<span class="tid"> · working…</span>{/if}
      </span>
      <button class="btn" type="button" onclick={editor.undo} disabled={!editor.state.can_undo || editor.busy} title="Ctrl+Z">Undo</button>
      <button class="btn" type="button" onclick={editor.redo} disabled={!editor.state.can_redo || editor.busy} title="Ctrl+Shift+Z">Redo</button>
      <button class="btn" type="button" onclick={() => (reviewing = true)} disabled={changed === 0}>Review</button>
      <button class="btn primary" type="button" onclick={editor.save} disabled={changed === 0 || editor.busy} title="Ctrl+S">Save</button>
      <button class="btn" type="button" onclick={discard} disabled={changed === 0 || editor.busy}>Discard</button>
      {#if editor.state.conflicts.length > 0}
        <p class="error">Changed on disk while you edited: {editor.state.conflicts.join(", ")}. Saving is refused; discard to read the new version.</p>
      {/if}
      {#if editor.error && !editor.target}<p class="error">{editor.error}</p>{/if}
    </div>
  </div>
{/if}
{#if reviewing}<DiffPanel close={() => (reviewing = false)} />{/if}
