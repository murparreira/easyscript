<script lang="ts">
  // The cast: who speaks, under which names, with which face. Speakers no character claims
  // come first, ready to become one or to join one.
  import type { CharacterView } from "./bindings/CharacterView";
  import { useEditor } from "./editor.svelte";
  import Face from "./Face.svelte";
  import { useReader } from "./reader.svelte";
  import { list, slug } from "./text";

  const reader = useReader();
  const editor = useEditor()!;
  const characters = $derived(reader.view.characters);
  const lines = $derived(new Map(reader.view.stats.speakers.map((s) => [s.name, s.count])));
  const claimed = $derived(new Set(characters.flatMap((c) => c.speakers)));
  const unclaimed = $derived(reader.view.stats.speakers.filter((s) => s.name && !claimed.has(s.name) && !s.name.includes("{{")));

  // Portraits: the images the face tags can name, and those already chosen.
  const faceTag = $derived(reader.view.tags.find((t) => t.show === "face" && t.kind === "asset"));
  const portraits = $derived([
    ...new Set([
      ...characters.map((c) => c.portrait).filter((p): p is string => !!p),
      ...(faceTag?.path ? faceTag.available.map((v) => faceTag.path!.replace("{value}", v)) : []),
    ]),
  ]);

  type Form = { id: string; name: string; speakers: string; portrait: string; role: string; isNew: boolean };
  let form: Form | null = $state(null);

  function open(c: CharacterView | null, speaker = "") {
    form = c
      ? { id: c.id, name: c.name, speakers: c.speakers.join(", "), portrait: c.portrait ?? "", role: c.role ?? "", isNew: false }
      : { id: slug(speaker), name: speaker, speakers: speaker, portrait: "", role: "", isNew: true };
  }

  async function save() {
    if (!form) return;
    const ok = await editor.editProject([
      { op: "set_character", id: form.id, name: form.name, speakers: list(form.speakers), portrait: form.portrait || null, role: form.role || null },
    ]);
    if (ok) form = null;
  }

  function join(c: CharacterView, speaker: string) {
    editor.editProject([{ op: "set_character", id: c.id, name: c.name, speakers: [...c.speakers, speaker], portrait: c.portrait, role: c.role }]);
  }

  const total = (c: CharacterView) => c.speakers.reduce((n, s) => n + (lines.get(s) ?? 0), 0);
</script>

<section class="sec builder" id="b-characters">
  <h2>Characters</h2>
  <p class="lede">Who speaks, under which names in the files, and the face their lines show when no tag picks another.</p>

  {#if unclaimed.length > 0}
    <div class="bnote">
      <p><b>Speakers no character claims</b> (checks warn about their lines):</p>
      <ul class="claims">
        {#each unclaimed as s (s.name)}
          <li>
            <b>{s.name}</b> <span class="tid">{s.count} lines</span>
            <button type="button" class="link" onclick={() => open(null, s.name)}>new character</button>
            {#if characters.length > 0}
              <select aria-label="Add {s.name} to a character" onchange={(e) => { const c = characters.find((x) => x.id === e.currentTarget.value); if (c) join(c, s.name); }}>
                <option value="">add to…</option>
                {#each characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
              </select>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="b-actions"><button class="btn" type="button" onclick={() => open(null)}>Add a character</button></div>

  {#if form?.isNew}{@render editForm()}{/if}

  <ol class="cards">
    {#each characters as c, i (c.id)}
      <li class="card">
        {#if form && !form.isNew && form.id === c.id}
          {@render editForm()}
        {:else}
          <div class="card-head">
            <Face image={c.portrait ? (reader.imageClasses.get(c.portrait) ?? null) : null} name={c.name} />
            <div>
              <h4>{c.name} <span class="tid">{c.id}</span></h4>
              <div class="tid">{c.speakers.join(" · ")} · {total(c)} lines</div>
            </div>
            <div class="card-tools">
              <button type="button" title="Edit" onclick={() => open(c)}>✎</button>
              <button type="button" title="Move up" disabled={i === 0} onclick={() => editor.editProject([{ op: "move_character", id: c.id, up: true }])}>↑</button>
              <button type="button" title="Move down" disabled={i === characters.length - 1} onclick={() => editor.editProject([{ op: "move_character", id: c.id, up: false }])}>↓</button>
              <button type="button" title="Remove" onclick={() => editor.editProject([{ op: "remove_character", id: c.id }])}>✕</button>
            </div>
          </div>
          {#if c.role}<p class="role">{c.role}</p>{/if}
          {@const faces = c.speakers.flatMap((s) => reader.facesBySpeaker.get(s) ?? [])}
          {#if faces.length > 0 && faceTag?.path}
            <div class="gallery" aria-label="Faces their lines use">
              {#each [...new Set(faces)] as face (face)}
                <span class="av face {reader.imageClasses.get(faceTag.path.replace('{value}', face)) ?? ''}" title={face}></span>
              {/each}
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
      <div class="le-title k">{form.isNew ? "New character" : `Edit ${form.id}`}</div>
      <div class="le-grid">
        <div class="le-fields">
          {#if form.isNew}<label class="le-field"><span class="k">Id</span><input bind:value={form.id} spellcheck="false" /></label>{/if}
          <label class="le-field"><span class="k">Name</span><input bind:value={form.name} /></label>
          <label class="le-field"><span class="k">Speaker names in the files (comma separated)</span><input bind:value={form.speakers} spellcheck="false" /></label>
          <label class="le-field"><span class="k">Role</span><textarea bind:value={form.role} rows="2"></textarea></label>
        </div>
        <div class="le-side">
          <span class="k">Portrait</span>
          <div class="face-grid">
            <button type="button" class="face-option" class:chosen={!form.portrait} title="none" onclick={() => form && (form.portrait = "")}><span class="av mono-av">—</span></button>
            {#each portraits as path (path)}
              <button type="button" class="face-option" class:chosen={form.portrait === path} title={path} onclick={() => form && (form.portrait = path)}>
                <span class="av face {reader.imageClasses.get(path) ?? ''}"></span>
              </button>
            {/each}
          </div>
          <input bind:value={form.portrait} placeholder="path/to/portrait.png" spellcheck="false" aria-label="Portrait path" />
        </div>
      </div>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="submit" disabled={editor.busy}>{form.isNew ? "Add" : "Apply"}</button>
        <button class="btn" type="button" onclick={() => (form = null)}>Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}
