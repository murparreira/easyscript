<script lang="ts">
  // The cast: who speaks, under which names, with which face. Speakers no character claims
  // come first, ready to become one or to join one.
  import type { CharacterView } from "./bindings/CharacterView";
  import { useEditor } from "./editor.svelte";
  import Face from "./Face.svelte";
  import { plural, t } from "./i18n.svelte";
  import { pickImage } from "./pickFile";
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

  type Form = { id: string; name: string; speakers: string; portrait: string; role: string; voices: string[]; isNew: boolean };
  const voiceTag = $derived(reader.view.tags.find((t) => t.show === "voice" && t.kind === "enum"));
  let form: Form | null = $state(null);

  function open(c: CharacterView | null, speaker = "") {
    form = c
      ? { id: c.id, name: c.name, speakers: c.speakers.join(", "), portrait: c.portrait ?? "", role: c.role ?? "", voices: [...c.voices], isNew: false }
      : { id: slug(speaker), name: speaker, speakers: speaker, portrait: "", role: "", voices: [], isNew: true };
  }

  async function save() {
    if (!form) return;
    const ok = await editor.editProject([
      { op: "set_character", id: form.id, name: form.name, speakers: list(form.speakers), portrait: form.portrait || null, role: form.role || null, voices: form.voices },
    ]);
    if (ok) form = null;
  }

  function join(c: CharacterView, speaker: string) {
    editor.editProject([{ op: "set_character", id: c.id, name: c.name, speakers: [...c.speakers, speaker], portrait: c.portrait, role: c.role, voices: c.voices }]);
  }

  const total = (c: CharacterView) => c.speakers.reduce((n, s) => n + (lines.get(s) ?? 0), 0);

  // A portrait from the files, with the system's file chooser.
  let pickError: string | null = $state(null);
  async function browsePortrait() {
    pickError = null;
    try {
      const picked = await pickImage(t("Choose a portrait"), form?.portrait || portraits[0]);
      if (picked && form) form.portrait = picked;
    } catch (e) {
      pickError = String(e);
    }
  }
</script>

<section class="sec builder" id="b-characters">
  <h2>{t("Characters")}</h2>
  <p class="lede">{t("Who speaks, under which names in the files, and the face their lines show when no tag picks another.")}</p>

  {#if unclaimed.length > 0}
    <div class="bnote">
      <p>{t("Speakers no character claims (checks warn about their lines):")}</p>
      <ul class="claims">
        {#each unclaimed as s (s.name)}
          <li>
            <b>{s.name}</b> <span class="tid">{plural(s.count, "{n} line", "{n} lines")}</span>
            <button type="button" class="link" onclick={() => open(null, s.name)}>{t("new character")}</button>
            {#if characters.length > 0}
              <select aria-label={t("Add {name} to a character", { name: s.name })} onchange={(e) => { const c = characters.find((x) => x.id === e.currentTarget.value); if (c) join(c, s.name); }}>
                <option value="">{t("add to…")}</option>
                {#each characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
              </select>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="b-actions"><button class="btn" type="button" onclick={() => open(null)}>{t("Add a character")}</button></div>

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
              <div class="tid">{c.speakers.join(" · ")} · {plural(total(c), "{n} line", "{n} lines")}</div>
            </div>
            <div class="card-tools">
              <button type="button" title={t("Edit")} onclick={() => open(c)}>✎</button>
              <button type="button" title={t("Move up")} disabled={i === 0} onclick={() => editor.editProject([{ op: "move_character", id: c.id, up: true }])}>↑</button>
              <button type="button" title={t("Move down")} disabled={i === characters.length - 1} onclick={() => editor.editProject([{ op: "move_character", id: c.id, up: false }])}>↓</button>
              <button type="button" title={t("Remove")} onclick={() => editor.editProject([{ op: "remove_character", id: c.id }])}>✕</button>
            </div>
          </div>
          {#if c.role}<p class="role">{c.role}</p>{/if}
          {#if c.voices.length > 0}<div class="values"><span class="k">{t("Voice")}</span>{#each c.voices as v (v)}<span class="vtag">{v}</span>{/each}</div>{/if}
          {@const faces = c.speakers.flatMap((s) => reader.facesBySpeaker.get(s) ?? [])}
          {#if faces.length > 0 && faceTag?.path}
            <div class="gallery" aria-label={t("Faces their lines use")}>
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
      <div class="le-title k">{form.isNew ? t("New character") : t("Edit {id}", { id: form.id })}</div>
      <div class="le-grid">
        <div class="le-fields">
          {#if form.isNew}<label class="le-field"><span class="k">{t("Id")}</span><input bind:value={form.id} spellcheck="false" /></label>{/if}
          <label class="le-field"><span class="k">{t("Name")}</span><input bind:value={form.name} /></label>
          <label class="le-field"><span class="k">{t("Speaker names in the files (comma separated)")}</span><input bind:value={form.speakers} spellcheck="false" /></label>
          <label class="le-field"><span class="k">{t("Role")}</span><textarea bind:value={form.role} rows="2"></textarea></label>
          {#if voiceTag}
            <fieldset class="voices"><legend class="k">{t("Voice moods recorded (none: not tracked)")}</legend>
              {#each voiceTag.values as v (v)}
                <label class="tog"><input type="checkbox" checked={form.voices.includes(v)} onchange={(e) => form && (form.voices = e.currentTarget.checked ? [...form.voices, v] : form.voices.filter((x) => x !== v))} /> {v}</label>
              {/each}
            </fieldset>
          {/if}
        </div>
        <div class="le-side">
          <span class="k">{t("Portrait")}</span>
          <div class="face-grid">
            <button type="button" class="face-option" class:chosen={!form.portrait} title={t("none")} onclick={() => form && (form.portrait = "")}><span class="av mono-av">—</span></button>
            {#each portraits as path (path)}
              <button type="button" class="face-option" class:chosen={form.portrait === path} title={path} onclick={() => form && (form.portrait = path)}>
                <span class="av face {reader.imageClasses.get(path) ?? ''}"></span>
              </button>
            {/each}
          </div>
          <div class="le-row">
            <input class="grow" bind:value={form.portrait} placeholder="path/to/portrait.png" spellcheck="false" aria-label={t("Portrait path")} />
            <button type="button" class="btn small" onclick={browsePortrait}>{t("Browse…")}</button>
          </div>
          {#if form.portrait && !reader.imageClasses.has(form.portrait)}<p class="hint">{t("The portrait shows once you apply.")}</p>{/if}
          {#if pickError}<p class="error">{pickError}</p>{/if}
        </div>
      </div>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="submit" disabled={editor.busy}>{form.isNew ? t("Add") : t("Apply")}</button>
        <button class="btn" type="button" onclick={() => (form = null)}>{t("Cancel")}</button>
      </div>
    </form>
  {/if}
{/snippet}
