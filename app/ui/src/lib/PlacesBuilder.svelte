<script lang="ts">
  // Places in the order a player meets them, and the files (or labels) each one plays.
  import type { PlaceView } from "./bindings/PlaceView";
  import { useEditor } from "./editor.svelte";
  import { useReader } from "./reader.svelte";
  import { list, slug } from "./text";

  const reader = useReader();
  const editor = useEditor()!;
  const files = $derived(reader.view.script.files.map((f) => f.path));
  const placed = $derived(new Set(reader.view.places.flatMap((p) => p.files)));
  const unplaced = $derived(files.filter((f) => !placed.has(f)));

  type Form = { id: string; name: string; group: string; files: string[]; labels: string; exclude: string; isNew: boolean };
  let form: Form | null = $state(null);

  function open(p: PlaceView | null, file = "") {
    form = p
      ? { id: p.id, name: p.name, group: p.group ?? "", files: [...p.files], labels: p.labels.join(", "), exclude: p.exclude_labels.join(", "), isNew: false }
      : { id: slug(file.split("/").pop()?.replace(/\.dialogue$/, "") ?? ""), name: "", group: "", files: file ? [file] : [], labels: "", exclude: "", isNew: true };
  }

  async function save() {
    if (!form) return;
    const ok = await editor.editProject([
      { op: "set_place", id: form.id, name: form.name, group: form.group || null, files: form.files, labels: list(form.labels), exclude_labels: list(form.exclude) },
    ]);
    if (ok) form = null;
  }

  function toggle(file: string, on: boolean) {
    if (!form) return;
    form.files = on ? [...form.files, file] : form.files.filter((f) => f !== file);
  }
</script>

<section class="sec builder" id="b-places">
  <h2>Places</h2>
  <p class="lede">The script is read by place, in this order; places with the same group are shown together.</p>
  {#if unplaced.length > 0}
    <div class="bnote"><p><b>Files in no place:</b>
      {#each unplaced as f (f)}<button type="button" class="link" onclick={() => open(null, f)}>{f}</button>{/each}</p></div>
  {/if}
  <div class="b-actions"><button class="btn" type="button" onclick={() => open(null)}>Add a place</button></div>
  {#if form?.isNew}{@render editForm()}{/if}
  <ol class="cards">
    {#each reader.view.places as p, i (p.id)}
      <li class="card">
        {#if form && !form.isNew && form.id === p.id}
          {@render editForm()}
        {:else}
          <div class="card-head">
            <div>
              {#if p.group}<div class="dist">{p.group}</div>{/if}
              <h4>{p.name} <span class="tid">{p.id}</span></h4>
              <div class="tid">{p.files.join(" · ")}{p.labels.length ? ` · only ${p.labels.join(", ")}` : ""}{p.exclude_labels.length ? ` · not ${p.exclude_labels.join(", ")}` : ""}</div>
            </div>
            <div class="card-tools">
              <button type="button" title="Edit" onclick={() => open(p)}>✎</button>
              <button type="button" title="Move up" disabled={i === 0} onclick={() => editor.editProject([{ op: "move_place", id: p.id, up: true }])}>↑</button>
              <button type="button" title="Move down" disabled={i === reader.view.places.length - 1} onclick={() => editor.editProject([{ op: "move_place", id: p.id, up: false }])}>↓</button>
              <button type="button" title="Remove" onclick={() => editor.editProject([{ op: "remove_place", id: p.id }])}>✕</button>
            </div>
          </div>
        {/if}
      </li>
    {/each}
  </ol>
</section>

{#snippet editForm()}
  {#if form}
    <form class="line-editor" onsubmit={(e) => { e.preventDefault(); save(); }}>
      <div class="le-title k">{form.isNew ? "New place" : `Edit ${form.id}`}</div>
      <div class="le-fields">
        {#if form.isNew}<label class="le-field"><span class="k">Id</span><input bind:value={form.id} spellcheck="false" /></label>{/if}
        <div class="le-row">
          <label class="le-field"><span class="k">Name</span><input bind:value={form.name} /></label>
          <label class="le-field"><span class="k">Group</span><input bind:value={form.group} placeholder="a district, a chapter" /></label>
        </div>
        <fieldset class="files"><legend class="k">Files</legend>
          {#each files as f (f)}<label class="tog"><input type="checkbox" checked={form.files.includes(f)} onchange={(e) => toggle(f, e.currentTarget.checked)} /> <code>{f}</code></label>{/each}
        </fieldset>
        <label class="le-field"><span class="k">Only these labels (comma separated, empty for all)</span><input bind:value={form.labels} spellcheck="false" /></label>
        <label class="le-field"><span class="k">All labels but these</span><input bind:value={form.exclude} spellcheck="false" /></label>
      </div>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="submit" disabled={editor.busy}>{form.isNew ? "Add" : "Apply"}</button>
        <button class="btn" type="button" onclick={() => (form = null)}>Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}
