<script lang="ts">
  // The scenes (labels) place by place, to pick one to write; a search through their lines;
  // a new scene. Scenes the team touched are marked: changed on origin and not here yet,
  // being changed on a teammate's branch, changed since the writer last looked.
  import { tick } from "svelte";
  import { useEditor } from "../editor.svelte";
  import { plural, t } from "../i18n.svelte";
  import { walk } from "../model";
  import { useReader } from "../reader.svelte";
  import { sceneKey, useTeam } from "../team.svelte";
  import { normalize } from "../text";
  import NewFile from "./NewFile.svelte";

  const reader = useReader();
  const editor = useEditor()!;
  const team = useTeam();

  let query = $state("");
  const q = $derived(normalize(query.trim()));
  /** Only the scenes the team touched. */
  let touchedOnly = $state(false);
  const keyOf = (id: string) => {
    const label = reader.byId.get(id);
    return label ? sceneKey(reader.view.script.files[label.span.file].path, id) : "";
  };
  const marks = (id: string) => {
    const key = keyOf(id);
    const names = (xs: { author: string }[] | undefined) => [...new Set((xs ?? []).map((x) => x.author))].join(", ");
    const incoming = team?.incoming.get(key);
    const others = team?.others.get(key);
    const since = team?.since.get(key);
    return {
      any: !!(incoming || others || since),
      incoming: incoming && t("{who} changed it on origin; not here yet", { who: names(incoming) }),
      others: others && t("{who} is changing it on {branch}", { who: names(others.map((o) => o.last)), branch: others.map((o) => o.branch).join(", ") }),
      since: since && t("{who} changed it since you last looked", { who: names(since) }),
    };
  };
  const touched = $derived.by(() => {
    const keys = new Set([...(team?.incoming.keys() ?? []), ...(team?.others.keys() ?? []), ...(team?.since.keys() ?? [])]);
    return keys;
  });
  const sinceCount = $derived(team?.since.size ?? 0);

  // A project with no project file: say what one gives, once.
  const SETUP = `dialogue-maker:setup-later:${reader.view.title}`;
  let later = $state((() => {
    try {
      return localStorage.getItem(SETUP) === "1";
    } catch {
      return false;
    }
  })());
  function notNow() {
    later = true;
    try {
      localStorage.setItem(SETUP, "1");
    } catch {
      // Asked again next time.
    }
  }
  async function setUp() {
    if (await editor.draftProject()) reader.tab = "project";
  }
  const sinceWho = $derived([...new Set((team?.state?.since ?? []).filter((c) => !c.mine).map((c) => c.author))].join(", "));
  const current = $derived(reader.current());

  const places = $derived(
    reader.groups.map((g) => ({
      name: g.name,
      places: g.places.map((p) => ({ id: p.id, name: p.name, labels: p.files.flatMap((f) => f.sections.flatMap((s) => s.labels)) })),
    })),
  );
  const matches = (id: string) => {
    if (touchedOnly && !touched.has(keyOf(id))) return false;
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

  // A new scene, after the current one in its file; or a new file.
  let filing = $state(false);
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
  {#if !editor.state.has_project_file && !later && reader.view.script.files.length > 0}
    <div class="catch-up setup">
      <p>{t("The app is guessing who speaks and where scenes play. A project file tells it, for the whole team: characters and their faces, places, flags.")}</p>
      <div class="le-row">
        <button class="btn small primary" type="button" onclick={setUp} disabled={editor.busy}>{t("Set it up")}</button>
        <button class="btn small" type="button" onclick={notNow}>{t("Not now")}</button>
      </div>
    </div>
  {/if}
  {#if sinceCount > 0}
    <div class="catch-up">
      <p>{plural(sinceCount, "{n} scene changed since you last looked", "{n} scenes changed since you last looked")}{#if sinceWho}, {t("by {who}", { who: sinceWho })}{/if}.</p>
      <div class="le-row">
        <button class="btn small" type="button" aria-pressed={touchedOnly} onclick={() => (touchedOnly = !touchedOnly)}>{touchedOnly ? t("Show all scenes") : t("Show them")}</button>
        <button class="btn small" type="button" onclick={() => { touchedOnly = false; team?.markSeen(); }}>{t("Got it")}</button>
      </div>
    </div>
  {:else if touched.size > 0}
    <div class="catch-up quiet">
      <button type="button" class="link" aria-pressed={touchedOnly} onclick={() => (touchedOnly = !touchedOnly)}>{touchedOnly ? t("Show all scenes") : plural(touched.size, "{n} scene the team is changing", "{n} scenes the team is changing")}</button>
    </div>
  {/if}
  <div class="rail-list">
    {#each places as group, g (g)}
      {#if group.name}<div class="rail-group">{group.name}</div>{/if}
      {#each group.places as place (place.id)}
        {@const shown = place.labels.filter((l) => matches(l.id))}
        {#if shown.length > 0}
          {@const expanded = open.has(place.id) || q.length > 1 || touchedOnly}
          {@const flagged = !expanded && place.labels.some((l) => touched.has(keyOf(l.id)))}
          <button type="button" class="rail-place" aria-expanded={expanded} onclick={() => toggle(place.id)}>
            <span class="caret" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
            <span class="name">{place.name}</span>
            {#if flagged}<span class="pip team" title={t("The team changed scenes here")}></span>{/if}
            <span class="count">{q.length > 1 ? `${shown.length}/${place.labels.length}` : place.labels.length}</span>
          </button>
          {#if expanded}
            <ul class="rail-scenes">
              {#each shown as label (label.id + label.span.file)}
                {@const m = marks(label.id)}
                <li>
                  <button type="button" class="rail-scene" aria-current={current?.id === label.id ? "true" : undefined} onclick={() => pick(label.id)} title={label.notes[0]?.text}>
                    <span class="id">{label.id}</span>
                    {#if m.any}
                      <span class="team-marks">
                        {#if m.incoming}<span class="tm in" title={m.incoming}>↓</span>{/if}
                        {#if m.others}<span class="tm other" title={m.others}>●</span>{/if}
                        {#if m.since}<span class="tm new" title={m.since}>•</span>{/if}
                      </span>
                    {/if}
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
    {:else if filing}
      <NewFile done={() => (filing = false)} />
    {:else}
      <div class="le-row">
        <button class="btn" type="button" onclick={startNaming} disabled={!current} title={current ? t("A new scene after {scene}", { scene: current.id }) : undefined}>{t("+ Scene")}</button>
        <button class="btn" type="button" onclick={() => (filing = true)} title={t("A new dialogue file")}>{t("+ File")}</button>
      </div>
    {/if}
  </div>
</nav>
