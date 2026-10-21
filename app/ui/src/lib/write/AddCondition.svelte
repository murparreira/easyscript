<script lang="ts">
  // A new `if`, `elif` or `else`, with the first line said under it.
  import ConditionInput from "../ConditionInput.svelte";
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";

  let {
    file,
    at,
    mode,
    keyword,
    done,
  }: { file: number; at: number; mode: "before" | "after" | "under"; keyword: "if" | "elif" | "else"; done: () => void } = $props();
  const editor = useEditor()!;
  const reader = useReader();

  let condition = $state("");
  let speaker = $state("");
  let words = $state("");
  const ready = $derived(words.trim() !== "" && (keyword === "else" || condition.trim() !== ""));

  async function add() {
    const path = reader.view.script.files[file].path;
    const line = { choice: false, speaker: speaker.trim() || null, text: words, tags: [], condition: null, jump: null };
    const edit = { op: "insert_condition" as const, at, after: mode === "after", under: mode === "under", keyword, condition: keyword === "else" ? null : condition.trim(), line };
    if ((await editor.edit(path, [edit])) !== null) done();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form class="add-form" onsubmit={(e) => { e.preventDefault(); add(); }} onkeydown={(e) => e.key === "Escape" && done()}>
  <div class="k">{keyword === "if" ? t("New condition") : keyword === "elif" ? t("Another case: or if") : t("Otherwise")}</div>
  {#if keyword !== "else"}<ConditionInput bind:value={condition} label={keyword === "if" ? t("If") : t("Or if")} />{/if}
  <div class="le-row">
    <label class="field inline"><span class="k">{t("Speaker")}</span>
      <input type="text" bind:value={speaker} list="dm-speakers" placeholder={t("narration")} spellcheck="false" />
    </label>
    <datalist id="dm-speakers">{#each reader.speakers as name (name)}<option value={name}></option>{/each}</datalist>
    <label class="field inline grow"><span class="k">{t("Says")}</span><input type="text" bind:value={words} placeholder={t("The first line said then")} /></label>
  </div>
  <div class="le-actions">
    <button class="btn primary small" type="submit" disabled={!ready || editor.busy}>{t("Add")}</button>
    <button class="btn small" type="button" onclick={done}>{t("Cancel")}</button>
  </div>
</form>
