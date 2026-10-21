<script lang="ts">
  // A new dialogue file and its first scene, where the project's other dialogue files are
  // (or `dialogue/`, the Dialogue Manager's habit, in a project with none yet). Unsaved
  // until the project is saved. When the project file lists places, the file can be one.
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, untrack } from "svelte";
  import type { Session } from "../bindings/Session";
  import { useEditor } from "../editor.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";

  let { done, first = false }: { done: () => void; first?: boolean } = $props();
  const editor = useEditor()!;
  const reader = useReader();

  /** The folder most dialogue files are in. */
  const folder = (() => {
    const counts = new Map<string, number>();
    for (const f of reader.view.script.files) {
      const dir = f.path.includes("/") ? f.path.slice(0, f.path.lastIndexOf("/")) : "";
      counts.set(dir, (counts.get(dir) ?? 0) + 1);
    }
    const best = [...counts].sort((a, b) => b[1] - a[1])[0];
    return best ? best[0] : "dialogue";
  })();

  let name = $state("");
  let scene = $state(untrack(() => (first ? "start" : "")));
  const stem = $derived(name.trim().replace(/\.dialogue$/, ""));
  const path = $derived(stem ? `${folder ? folder + "/" : ""}${stem}.dialogue` : "");
  const sceneName = $derived(scene.trim() || stem);
  const valid = $derived(/^[A-Za-z0-9_][A-Za-z0-9_\-/]*$/.test(stem) && /^[A-Za-z_][A-Za-z0-9_]*$/.test(sceneName));
  let error: string | null = $state(null);

  // A place for it, when the project has places (else each file is one already).
  const hasPlaces = $derived(reader.view.places.length > 0);
  const groups = $derived([...new Set(reader.view.places.map((p) => p.group).filter((g): g is string => !!g))]);
  const SMALL = new Set(["de", "da", "do", "das", "dos", "e", "a", "o", "of", "the", "and"]);
  const humane = (stem: string) =>
    stem
      .split(/[_\-\s]+/)
      .filter(Boolean)
      .map((w, i) => (i > 0 && SMALL.has(w) ? w : w.charAt(0).toUpperCase() + w.slice(1)))
      .join(" ");
  let asPlace = $state(true);
  let placeName = $state("");
  let group = $state("");
  const shownPlaceName = $derived(placeName.trim() || humane(stem.split("/").pop() ?? ""));
  let box: HTMLInputElement | undefined = $state();
  onMount(() => box?.focus());

  async function create() {
    error = null;
    editor.busy = true;
    try {
      editor.receive(await invoke<Session>("new_dialogue_file", { path, scene: sceneName }));
      if (hasPlaces && asPlace) {
        const id = (stem.split("/").pop() ?? stem).toLowerCase();
        await editor.editProject([{ op: "set_place", id, name: shownPlaceName, group: group || null, files: [path], labels: [], exclude_labels: [] }]);
      }
      reader.scene = sceneName;
      done();
    } catch (e) {
      error = String(e);
    } finally {
      editor.busy = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form class="new-file" onsubmit={(e) => { e.preventDefault(); create(); }} onkeydown={(e) => e.key === "Escape" && !first && done()}>
  <label class="field">
    <span class="k">{t("File name")}</span>
    <input bind:this={box} bind:value={name} placeholder={first ? "intro" : t("a place, a character, a chapter")} spellcheck="false" />
  </label>
  <label class="field">
    <span class="k">{t("First scene")}</span>
    <input bind:value={scene} placeholder={stem || "start"} spellcheck="false" />
  </label>
  {#if hasPlaces}
    <label class="tog"><input type="checkbox" bind:checked={asPlace} /> {t("A place of its own in the project")}</label>
    {#if asPlace}
      <div class="le-row">
        <label class="field inline grow"><span class="k">{t("Place")}</span><input bind:value={placeName} placeholder={shownPlaceName || t("Avenida Bandeirantes")} /></label>
        {#if groups.length > 0}
          <label class="field inline"><span class="k">{t("Group")}</span>
            <select bind:value={group}><option value="">—</option>{#each groups as g (g)}<option value={g}>{g}</option>{/each}</select>
          </label>
        {/if}
      </div>
    {/if}
  {/if}
  {#if path}<p class="hint">{t("Makes {path}, written when you save.", { path })}</p>{/if}
  {#if stem && !valid}<p class="warn">{t("Letters, digits and _, not starting with a digit.")}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="le-actions">
    <button class="btn primary" type="submit" disabled={!valid || editor.busy}>{t("Create")}</button>
    {#if !first}<button class="btn" type="button" onclick={done}>{t("Cancel")}</button>{/if}
  </div>
</form>
