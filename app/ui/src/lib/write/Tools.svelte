<script lang="ts">
  // What can be done to a step from where it is: open it, move it, add after or inside it,
  // remove it. A jump's target changes here too.
  import type { Jump } from "../bindings/Jump";
  import type { Span } from "../bindings/Span";
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";
  import JumpPicker from "./JumpPicker.svelte";

  let {
    span,
    choice,
    edit = true,
    nested = false,
    jump = null,
  }: { span: Span; choice: boolean; edit?: boolean; nested?: boolean; jump?: Jump | null } = $props();
  const editor = useEditor()!;
  const reader = useReader();
  const path = $derived(reader.view.script.files[span.file].path);

  function open(mode: "edit" | "after" | "under", asChoice = choice) {
    editor.target = { file: span.file, at: span.start, mode, choice: asChoice };
  }
  const move = (up: boolean) => editor.edit(path, [{ op: "move_line", at: span.start, up }]);
  const remove = () => editor.edit(path, [{ op: "remove_line", at: span.start }]);

  // A jump's target, changed in place.
  let retargeting = $state(false);
  let target = $state("");
  function startRetarget() {
    if (!jump) return;
    target = jump.target.kind === "label" ? jump.target.value : jump.target.kind === "end" ? "END" : jump.target.kind === "end_conversation" ? "END!" : "";
    retargeting = true;
  }
  async function retarget() {
    if ((await editor.edit(path, [{ op: "set_jump", at: span.start, target }])) !== null) retargeting = false;
  }
</script>

{#if retargeting}
  <form class="retarget" onsubmit={(e) => { e.preventDefault(); retarget(); }}>
    <JumpPicker bind:value={target} none={false} label={t("Goes")} />
    <button class="btn primary small" type="submit" disabled={editor.busy}>{t("Apply")}</button>
    <button class="btn small" type="button" onclick={() => (retargeting = false)}>{t("Cancel")}</button>
  </form>
{:else}
  <div class="tools" role="toolbar" aria-label={choice ? t("Choice") : t("Line")}>
    {#if edit}<button type="button" class="icon" title={t("Edit")} aria-label={t("Edit")} onclick={() => open("edit")}>✎</button>{/if}
    {#if jump && jump.target.kind !== "expression"}<button type="button" class="icon" title={t("Go somewhere else")} aria-label={t("Go somewhere else")} onclick={startRetarget}>✎</button>{/if}
    <button type="button" class="icon" title={t("Move up")} aria-label={t("Move up")} onclick={() => move(true)}>↑</button>
    <button type="button" class="icon" title={t("Move down")} aria-label={t("Move down")} onclick={() => move(false)}>↓</button>
    {#if nested}<button type="button" class="icon text" title={t("Add a line said after this choice is picked")} onclick={() => open("under", false)}>{t("+ inside")}</button>{/if}
    <button type="button" class="icon text" title={choice ? t("Add a choice below") : t("Add a line below")} onclick={() => open("after", choice)}>{choice ? t("+ choice") : t("+ line")}</button>
    <button type="button" class="icon danger" title={t("Remove (with what is under it)")} aria-label={t("Remove")} onclick={remove}>✕</button>
  </div>
{/if}
