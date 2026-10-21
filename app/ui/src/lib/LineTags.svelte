<script lang="ts">
  // A line's tags that have no control of their own (the thought and shake switches, the
  // voice and the face have theirs): each with its value to change or remove, and a tag to
  // add, one the project or the script knows, or a new one the game will have to learn.
  import { tick } from "svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  let { tags = $bindable(), shown }: { tags: Record<string, string | null>; shown: Set<string> } = $props();
  const reader = useReader();

  const listed = $derived(Object.keys(tags).filter((name) => !shown.has(name)));
  /** The values a tag takes: the project file's, else the ones the script uses. */
  const valuesOf = (name: string) => {
    const spec = reader.tagSpec(name);
    if (spec?.kind === "enum" && spec.values.length > 0) return spec.values;
    if (spec?.kind === "asset" && spec.available.length > 0) return spec.available;
    return reader.tagUses.get(name) ?? [];
  };
  const takesNoValue = (name: string) => reader.tagSpec(name)?.kind === "flag" || (!reader.tagSpec(name) && reader.tagUses.has(name) && valuesOf(name).length === 0);
  const known = $derived(
    [...new Set([...reader.view.tags.map((s) => s.name), ...reader.tagUses.keys()])].filter((name) => !(name in tags)).sort(),
  );

  let adding = $state(false);
  let name = $state("");
  let value = $state("");
  let nameBox: HTMLInputElement | undefined = $state();
  const cleanName = $derived(name.trim().replace(/^#/, ""));
  const validName = $derived(/^[^\s=,\]\[]+$/.test(cleanName));
  const isNew = $derived(validName && !reader.tagSpec(cleanName) && !reader.tagUses.has(cleanName));

  async function open() {
    adding = true;
    name = "";
    value = "";
    await tick();
    nameBox?.focus();
  }
  function add() {
    if (!validName) return;
    tags[cleanName] = takesNoValue(cleanName) ? null : value.trim() || null;
    adding = false;
  }
  function keydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      add();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      adding = false;
    }
  }
</script>

{#each listed as tag (tag)}
  {@const values = valuesOf(tag)}
  <span class="tag-edit" title={reader.tagSpec(tag)?.description ?? ""}>
    <span class="tag-name">{tag}</span>
    {#if !takesNoValue(tag)}
      {#if reader.tagSpec(tag)?.kind === "enum" && values.length > 0}
        <select value={tags[tag] ?? ""} onchange={(e) => (tags[tag] = e.currentTarget.value || null)} aria-label={t("Value of {name}", { name: tag })}>
          <option value="">—</option>
          {#each values as v (v)}<option value={v}>{v}</option>{/each}
          {#if tags[tag] && !values.includes(tags[tag] ?? "")}<option value={tags[tag]}>{tags[tag]}</option>{/if}
        </select>
      {:else}
        <input
          type="text"
          value={tags[tag] ?? ""}
          oninput={(e) => (tags[tag] = e.currentTarget.value.trim() || null)}
          list="dm-tag-values-{tag}"
          size={Math.max(4, Math.min(24, (tags[tag] ?? "").length + 1))}
          spellcheck="false"
          aria-label={t("Value of {name}", { name: tag })}
        />
        <datalist id="dm-tag-values-{tag}">{#each values as v (v)}<option value={v}></option>{/each}</datalist>
      {/if}
    {/if}
    <button type="button" class="link" aria-label={t("Remove {name}", { name: tag })} title={t("Remove {name}", { name: tag })} onclick={() => delete tags[tag]}>✕</button>
  </span>
{/each}

{#if adding}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="tag-add" onkeydown={keydown}>
    <input bind:this={nameBox} bind:value={name} list="dm-tag-names" placeholder={t("tag")} spellcheck="false" aria-label={t("Tag name")} size="12" />
    <datalist id="dm-tag-names">{#each known as k (k)}<option value={k}></option>{/each}</datalist>
    {#if validName && !takesNoValue(cleanName)}
      <input bind:value list="dm-tag-new-values" placeholder={t("value (optional)")} spellcheck="false" aria-label={t("Value")} size="12" />
      <datalist id="dm-tag-new-values">{#each valuesOf(cleanName) as v (v)}<option value={v}></option>{/each}</datalist>
    {/if}
    <button type="button" class="btn small" onclick={add} disabled={!validName}>{t("Add")}</button>
    <button type="button" class="link" onclick={() => (adding = false)}>{t("Cancel")}</button>
  </span>
  {#if isNew}<p class="hint tag-new">{t("A new tag: the game's dialogue code has to know what to do with it. Say what it means, and how the app shows it, in Project › Tags.")}</p>{/if}
{:else}
  <button type="button" class="chip-btn" onclick={open} title={t("Add a tag to this line: one the script uses, or a new one")}>{t("+ Tag")}</button>
{/if}
