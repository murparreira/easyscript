<script lang="ts">
  // At the end of a scene: add a line, choices, a flag set, a condition, or a jump to
  // another scene.
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";
  import AddCode from "./AddCode.svelte";
  import AddCondition from "./AddCondition.svelte";
  import JumpPicker from "./JumpPicker.svelte";

  let { file, at, mode }: { file: number; at: number; mode: "before" | "after" } = $props();
  const editor = useEditor()!;
  const reader = useReader();

  const open = (choice: boolean) => {
    adding = null;
    editor.target = { file, at, mode, choice };
  };
  let adding: "jump" | "code" | "condition" | null = $state(null);
  let target = $state("END");
  async function addJump() {
    const path = reader.view.script.files[file].path;
    const line = { choice: false, speaker: null, text: "", tags: [], condition: null, jump: target };
    if ((await editor.edit(path, [{ op: "insert_line", at, after: mode === "after", under: false, line }])) !== null) adding = null;
  }
</script>

<div class="add-bar">
  <button class="btn" type="button" onclick={() => open(false)}>{t("+ Line")}</button>
  <button class="btn" type="button" onclick={() => open(true)}>{t("+ Choices")}</button>
  <button class="btn" type="button" aria-pressed={adding === "code"} onclick={() => (adding = adding === "code" ? null : "code")}>{t("+ Flag…")}</button>
  <button class="btn" type="button" aria-pressed={adding === "condition"} onclick={() => (adding = adding === "condition" ? null : "condition")}>{t("+ If…")}</button>
  <button class="btn" type="button" aria-pressed={adding === "jump"} onclick={() => (adding = adding === "jump" ? null : "jump")}>{t("+ Go to…")}</button>
</div>
{#if adding === "jump"}
  <form class="add-form" onsubmit={(e) => { e.preventDefault(); addJump(); }}>
    <JumpPicker bind:value={target} none={false} label={t("Then")} />
    <div class="le-actions">
      <button class="btn primary small" type="submit" disabled={editor.busy}>{t("Add")}</button>
      <button class="btn small" type="button" onclick={() => (adding = null)}>{t("Cancel")}</button>
    </div>
  </form>
{:else if adding === "code"}
  <AddCode {file} {at} {mode} done={() => (adding = null)} />
{:else if adding === "condition"}
  <AddCondition {file} {at} {mode} keyword="if" done={() => (adding = null)} />
{/if}
