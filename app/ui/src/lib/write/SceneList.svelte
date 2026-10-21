<script lang="ts">
  // The scenes (labels) place by place, to pick one to write; a search through their lines;
  // a new scene.
  import { tick } from "svelte";
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { walk } from "../model";
  import { useReader } from "../reader.svelte";
  import { normalize } from "../text";

  const reader = useReader();
  const editor = useEditor()!;

  let query = $state("");
  const q = $derived(normalize(query.trim()));
  const current = $derived(reader.current());

  const places = $derived(
    reader.groups.map((g) => ({
      name: g.name,
      places: g.places.map((p) => ({ id: p.id, name: p.name, labels: p.files.flatMap((f) => f.sections.flatMap((s) => s.labels)) })),
    })),
  );
  const matches = (id: string) => {
    if (q.length < 2) return true;
    const label = reader.byId.get(id);
    return label ? (reader.labelTexts.get(label) ?? "").includes(q) : false;
  };
  const lines = (id: string) => {
    const label = reader.byId.get(id);
    let n = 0;
    if (label) for (const node of walk(label.body)) if (node.type === "line") n++;
    return n;
  };

  // Places open in the list: the current scene's, and any the writer opened.
  let open: Set<string> = $state(new Set());
  const placeOf = (id: string) => places.flatMap((g) => g.places).find((p) => p.labels.some((l) => l.id === id))?.id;
  $effect(() => {
    const place = current ? placeOf(current.id) : undefined;
    if (place && !open.has(place)) open = new Set([...open, place]);
  });
  function toggle(place: string) {
    const next = new Set(open);
    if (next.has(place)) next.delete(place);
    else next.add(place);
    open = next;
  }

  function pick(id: string) {
    reader.scene = id;
    editor.target = null;
  }

  // A new scene, after the current one in its file.
  let naming = $state(false);
  let name = $state("");
  let nameBox: HTMLInputElement | undefined = $state();
  const valid = $derived(/^[A-Za-z_][A-Za-z0-9_]*$/.test(name.trim()));
  async function startNaming() {
    naming = true;
    name = "";
    await tick();
    nameBox?.focus();
  }
  async function addScene() {
    if (!current || !valid) return;
    const id = name.trim();
    const path = reader.view.script.files[current.span.file].path;
    if ((await editor.edit(path, [{ op: "add_label", at: current.span.start, id }])) !== null) {
      naming = false;
      reader.scene = id;
    }
  }
</script>

<nav class="rail" aria-label={t("Scenes")}>
  <div class="rail-search">
    <input type="search" placeholder={t("Find a scene or a line…")} bind:value={query} aria-label={t("Find a scene or a line")} />
  </div>
  <div class="rail-list">
    {#each places as group, g (g)}
      {#if group.name}<div class="rail-group">{group.name}</div>{/if}
      {#each group.places as place (place.id)}
        {@const shown = place.labels.filter((l) => matches(l.id))}
        {#if shown.length > 0}
          {@const expanded = open.has(place.id) || q.length > 1}
          <button type="button" class="rail-place" aria-expanded={expanded} onclick={() => toggle(place.id)}>
            <span class="caret" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
            <span class="name">{place.name}</span>
            <span class="count">{q.length > 1 ? `${shown.length}/${place.labels.length}` : place.labels.length}</span>
          </button>
          {#if expanded}
            <ul class="rail-scenes">
              {#each shown as label (label.id + label.span.file)}
                <li>
                  <button type="button" class="rail-scene" aria-current={current?.id === label.id ? "true" : undefined} onclick={() => pick(label.id)} title={label.notes[0]?.text}>
                    <span class="id">{label.id}</span>
                    <span class="count">{lines(label.id)}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      {/each}
    {/each}
  </div>
  <div class="rail-foot">
    {#if naming}
      <form class="new-scene" onsubmit={(e) => { e.preventDefault(); addScene(); }}>
        <input bind:this={nameBox} bind:value={name} placeholder={t("scene_name")} spellcheck="false" aria-label={t("Name of the new scene")}
          onkeydown={(e) => e.key === "Escape" && (naming = false)} />
        <button class="btn primary" type="submit" disabled={!valid || editor.busy}>{t("Add")}</button>
        <button class="btn" type="button" onclick={() => (naming = false)}>{t("Cancel")}</button>
        <p class="hint">{name && !valid ? t("Letters, digits and _, not starting with a digit.") : t("Goes after {scene}, in {file}.", { scene: current?.id ?? "", file: current ? reader.view.script.files[current.span.file].path : "" })}</p>
      </form>
    {:else}
      <button class="btn" type="button" onclick={startNaming} disabled={!current}>{t("+ Scene")}</button>
    {/if}
  </div>
</nav>
