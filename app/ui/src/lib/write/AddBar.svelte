<script lang="ts">
  // At the end of a scene: add a line, a choice, or a jump to another scene.
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";
  import JumpPicker from "./JumpPicker.svelte";

  let { file, at, mode }: { file: number; at: number; mode: "before" | "after" } = $props();
  const editor = useEditor()!;
  const reader = useReader();

  const open = (choice: boolean) => (editor.target = { file, at, mode, choice });
  let jumping = $state(false);
  let target = $state("END");
  async function addJump() {
    const path = reader.view.script.files[file].path;
    const line = { choice: false, speaker: null, text: "", tags: [], condition: null, jump: target };
    if ((await editor.edit(path, [{ op: "insert_line", at, after: mode === "after", under: false, line }])) !== null) jumping = false;
  }
</script>

<div class="add-bar">
  {#if jumping}
    <form class="retarget" onsubmit={(e) => { e.preventDefault(); addJump(); }}>
      <JumpPicker bind:value={target} none={false} label={t("Then")} />
      <button class="btn primary small" type="submit" disabled={editor.busy}>{t("Add")}</button>
      <button class="btn small" type="button" onclick={() => (jumping = false)}>{t("Cancel")}</button>
    </form>
  {:else}
    <button class="btn" type="button" onclick={() => open(false)}>{t("+ Line")}</button>
    <button class="btn" type="button" onclick={() => open(true)}>{t("+ Choices")}</button>
    <button class="btn" type="button" onclick={() => (jumping = true)}>{t("+ Go to…")}</button>
  {/if}
</div>
