<script lang="ts">
  // The tag schema: each tag's kind, values and how the views show it, with what the script
  // does with it.
  import type { TagKind } from "./bindings/TagKind";
  import type { TagView } from "./bindings/TagView";
  import { useEditor } from "./editor.svelte";
  import { walk } from "./model";
  import { useReader } from "./reader.svelte";
  import { list } from "./text";

  const reader = useReader();
  const editor = useEditor()!;

  // How the script uses each tag: lines, and each value's count.
  const usage = $derived.by(() => {
    const out = new Map<string, { lines: number; values: Map<string, number> }>();
    for (const file of reader.view.script.files) {
      for (const label of file.sections.flatMap((s) => s.labels)) {
        for (const node of walk(label.body)) {
          const lines = node.type === "line" ? [node] : node.type === "menu" ? node.choices : [];
          for (const line of lines) {
            for (const tag of line.tags) {
              const u = out.get(tag.name) ?? { lines: 0, values: new Map() };
              u.lines += 1;
              if (tag.value) u.values.set(tag.value, (u.values.get(tag.value) ?? 0) + 1);
              out.set(tag.name, u);
            }
          }
        }
      }
    }
    return out;
  });
  const missing = $derived([...usage.keys()].filter((name) => !reader.tagSpecs.has(name)));

  type Form = { name: string; kind: TagKind; values: string; path: string; show: string; description: string; isNew: boolean };
  let form: Form | null = $state(null);

  function open(tag: TagView | null, name = "") {
    if (tag) {
      form = { name: tag.name, kind: tag.kind, values: tag.values.join(", "), path: tag.path ?? "", show: tag.show ?? "", description: tag.description ?? "", isNew: false };
    } else {
      const used = usage.get(name);
      const values = used ? [...used.values.keys()] : [];
      form = { name, kind: values.length === 0 ? "flag" : "enum", values: values.join(", "), path: "", show: "", description: "", isNew: true };
    }
  }

  async function save() {
    if (!form) return;
    const ok = await editor.editProject([
      { op: "set_tag", name: form.name, kind: form.kind, values: list(form.values), path: form.path || null, show: form.show || null, description: form.description || null },
    ]);
    if (ok) form = null;
  }

  const ordered = (values: Map<string, number>) => [...values].sort((a, b) => b[1] - a[1]);
</script>

<section class="sec builder" id="b-tags">
  <h2>Tags</h2>
  <p class="lede">Line tags (<code>[#name=value]</code>): what each can hold, and how the views show it.</p>

  {#if missing.length > 0}
    <div class="bnote">
      <p><b>Tags the script uses that the schema does not have:</b>
        {#each missing as name (name)}<button type="button" class="link" onclick={() => open(null, name)}>{name}</button>{/each}
      </p>
    </div>
  {/if}
  <div class="b-actions"><button class="btn" type="button" onclick={() => open(null)}>Add a tag</button></div>
  {#if form?.isNew}{@render editForm()}{/if}

  <ol class="cards">
    {#each reader.view.tags as tag (tag.name)}
      {@const used = usage.get(tag.name)}
      <li class="card">
        {#if form && !form.isNew && form.name === tag.name}
          {@render editForm()}
        {:else}
          <div class="card-head">
            <div>
              <h4><code>{tag.name}</code> <span class="chip">{tag.kind}</span>{#if tag.show}<span class="chip">shows as {tag.show}</span>{/if}</h4>
              <div class="tid">on {used?.lines ?? 0} lines{tag.path ? ` · ${tag.path}` : ""}</div>
            </div>
            <div class="card-tools">
              <button type="button" title="Edit" onclick={() => open(tag)}>✎</button>
              <button type="button" title="Remove from the schema" onclick={() => editor.editProject([{ op: "remove_tag", name: tag.name }])}>✕</button>
            </div>
          </div>
          {#if tag.description}<p class="role">{tag.description}</p>{/if}
          {#if tag.kind === "enum"}
            <div class="values">
              {#each tag.values as v (v)}<span class="chip">{v} <span class="tid">{used?.values.get(v) ?? 0}</span></span>{/each}
              {#each ordered(used?.values ?? new Map()).filter(([v]) => !tag.values.includes(v)) as [v, n] (v)}
                <span class="chip bad" title="Not one of its values">{v} <span class="tid">{n}</span></span>
              {/each}
            </div>
          {:else if tag.kind === "asset" && tag.show === "face" && tag.path}
            <div class="gallery" aria-label="Its files">
              {#each tag.available.slice(0, 60) as v (v)}
                <span class="av face {reader.imageClasses.get(tag.path.replace('{value}', v)) ?? ''}" title="{v}: {used?.values.get(v) ?? 0} lines"></span>
              {/each}
              {#if tag.available.length > 60}<span class="tid">and {tag.available.length - 60} more</span>{/if}
            </div>
          {/if}
        {/if}
      </li>
    {/each}
  </ol>
</section>

{#snippet editForm()}
  {#if form}
    <form class="line-editor" onsubmit={(e) => { e.preventDefault(); save(); }}>
      <div class="le-title k">{form.isNew ? "New tag" : `Edit ${form.name}`}</div>
      <div class="le-fields">
        {#if form.isNew}<label class="le-field"><span class="k">Name</span><input bind:value={form.name} spellcheck="false" /></label>{/if}
        <div class="le-row">
          <label class="le-field inline"><span class="k">Kind</span>
            <select bind:value={form.kind}>
              <option value="flag">flag: present or not</option>
              <option value="enum">enum: one of some values</option>
              <option value="asset">asset: names a file</option>
              <option value="text">text: anything</option>
            </select>
          </label>
          <label class="le-field inline"><span class="k">Shows as</span>
            <select bind:value={form.show}>
              <option value="">nothing special</option>
              <option value="face">the line's face</option>
              <option value="voice">a voice mood</option>
              <option value="thought">a thought</option>
              <option value="shake">a shake</option>
            </select>
          </label>
        </div>
        {#if form.kind === "enum"}<label class="le-field"><span class="k">Values (comma separated)</span><input bind:value={form.values} spellcheck="false" /></label>{/if}
        {#if form.kind === "asset"}<label class="le-field"><span class="k">Where its file is, with {"{value}"}</span><input bind:value={form.path} placeholder="assets/faces/{'{value}'}.png" spellcheck="false" /></label>{/if}
        <label class="le-field"><span class="k">Description</span><input bind:value={form.description} /></label>
      </div>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="submit" disabled={editor.busy}>{form.isNew ? "Add" : "Apply"}</button>
        <button class="btn" type="button" onclick={() => (form = null)}>Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}
