<script lang="ts">
  // In edit mode, what can be done to a line or choice from where it is.
  import type { Span } from "./bindings/Span";
  import { useEditor } from "./editor.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  let { span, choice }: { span: Span; choice: boolean } = $props();
  const editor = useEditor()!;
  const reader = useReader();
  const path = $derived(reader.view.script.files[span.file].path);

  function open(mode: "edit" | "before" | "after", asChoice = choice) {
    editor.target = { file: span.file, at: span.start, mode, choice: asChoice };
  }
  const move = (up: boolean) => editor.edit(path, [{ op: "move_line", at: span.start, up }]);
  const remove = () => editor.edit(path, [{ op: "remove_line", at: span.start }]);
</script>

<div class="line-tools" role="toolbar" aria-label={t("Line")}>
  <button type="button" title={t("Edit")} onclick={() => open("edit")}>✎</button>
  <button type="button" title={t("Move up")} onclick={() => move(true)}>↑</button>
  <button type="button" title={t("Move down")} onclick={() => move(false)}>↓</button>
  <button type="button" title={t("Add a line below")} onclick={() => open("after", false)}>{t("+ line")}</button>
  <button type="button" title={t("Add a choice below")} onclick={() => open("after", true)}>{t("+ choice")}</button>
  <button type="button" title={t("Remove (with what is under it)")} onclick={remove}>✕</button>
</div>
