<script lang="ts">
  // A new line of the game's code: a flag set (the way the script sets flags), or any
  // `do` or `set` line.
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";

  let { file, at, mode, done }: { file: number; at: number; mode: "before" | "after" | "under"; done: () => void } = $props();
  const editor = useEditor()!;
  const reader = useReader();

  let flag = $state("");
  let other = $state(!reader.flagSet);
  let code = $state("do ");
  const written = $derived(other ? code.trim() : reader.flagSet && flag.trim() ? reader.flagSet.replace("{flag}", flag.trim()) : "");
  const known = $derived(new Set(reader.view.flags.map((f) => f.flag)));

  async function add() {
    const path = reader.view.script.files[file].path;
    const edit = { op: "insert_code" as const, at, after: mode === "after", under: mode === "under", code: written };
    if ((await editor.edit(path, [edit])) !== null) done();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form class="add-form" onsubmit={(e) => { e.preventDefault(); add(); }} onkeydown={(e) => e.key === "Escape" && done()}>
  <div class="k">{other ? t("Game code") : t("Set a flag")}</div>
  {#if other}
    <label class="field"><span class="k">{t("A line of the game's code")}</span><input type="text" bind:value={code} spellcheck="false" class="code" /></label>
  {:else}
    <label class="field">
      <span class="k">{t("Flag")}</span>
      <input type="text" bind:value={flag} list="dm-flags" spellcheck="false" placeholder={t("a flag, or a new name")} />
      <datalist id="dm-flags">{#each reader.view.flags as f (f.flag)}<option value={f.flag}>{f.meaning ?? ""}</option>{/each}</datalist>
    </label>
    {#if flag.trim() && !known.has(flag.trim())}<p class="hint">{t("A new flag: say what it means in Project › Flags.")}</p>{/if}
  {/if}
  {#if written}<p class="hint">{t("Writes")} <code>{written}</code></p>{/if}
  <div class="le-actions">
    <button class="btn primary small" type="submit" disabled={!written || written === "do" || editor.busy}>{t("Add")}</button>
    <button class="btn small" type="button" onclick={done}>{t("Cancel")}</button>
    {#if reader.flagSet}
      <button type="button" class="link" onclick={() => (other = !other)}>{other ? t("Set a flag instead") : t("Other game code…")}</button>
    {/if}
  </div>
</form>
