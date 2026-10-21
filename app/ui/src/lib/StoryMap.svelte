<script lang="ts">
  // The story at a glance: labels place by place, the jumps and flags between them, where
  // the game starts them; who speaks where; which flags each place sets and reads. The
  // walkthrough beside the map.
  import { marked } from "marked";
  import { untrack } from "svelte";
  import type { FlowKind } from "./bindings/FlowKind";
  import type { Label } from "./bindings/Label";
  import { walk } from "./model";
  import { plural, t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  const reader = useReader();
  let view: "flow" | "people" | "flags" = $state("flow");

  // Layout: a column per place, its labels down it in script order.
  const COL = 210;
  const GAP = 70;
  const ROW = 26;
  const HEAD = 52;
  const NODE_H = 20;

  type Column = { id: string; name: string; group: string | null; labels: Label[] };
  const columns: Column[] = $derived(
    reader.groups.flatMap((g) =>
      g.places.map((p) => ({ id: p.id, name: p.name, group: p.group, labels: p.files.flatMap((f) => f.sections.flatMap((s) => s.labels)) })),
    ),
  );
  const keyOf = (l: Label) => `${l.span.file}:${l.id}`;
  const positions = $derived.by(() => {
    const out = new Map<string, { x: number; y: number; label: Label; column: number }>();
    columns.forEach((c, i) => c.labels.forEach((l, j) => out.set(keyOf(l), { x: i * (COL + GAP) + 10, y: HEAD + j * ROW, label: l, column: i })));
    return out;
  });
  const width = $derived(columns.length * (COL + GAP) + 20);
  const height = $derived(HEAD + Math.max(1, ...columns.map((c) => c.labels.length)) * ROW + 20);
  const entries = $derived(new Set(reader.view.flow.entries));
  // Labels nothing leads to: not started by the game, and no link comes in.
  const reached = $derived(new Set([...reader.view.flow.entries, ...reader.view.flow.edges.map((e) => e.to)]));

  // The kinds of links between scenes, as the key tells them; which ones are drawn.
  const KINDS: Array<{ kind: FlowKind; name: () => string; says: () => string }> = [
    { kind: "jump", name: () => t("Goes to"), says: () => t("The scene ends by going to another one (=> scene).") },
    { kind: "choice", name: () => t("A choice leads to"), says: () => t("A choice the player picks goes there.") },
    { kind: "conditional", name: () => t("Only if…"), says: () => t("Goes there only when a condition holds.") },
    { kind: "return", name: () => t("Visits and comes back"), says: () => t("Plays the other scene, then carries on here (=>< scene).") },
    { kind: "fall_through", name: () => t("Runs on into"), says: () => t("No jump at its end, so the next scene in the file follows.") },
    { kind: "flag", name: () => t("Sets a flag read by"), says: () => t("A flag this scene sets is read by the other.") },
  ];
  const kindName = (kind: FlowKind) => KINDS.find((k) => k.kind === kind)?.name() ?? kind;
  let shown: Record<string, boolean> = $state(Object.fromEntries(KINDS.map((k) => [k.kind, true])));
  const counts = $derived(new Map(KINDS.map((k) => [k.kind, reader.view.flow.edges.filter((e) => e.kind === k.kind).length])));
  const edges = $derived(reader.view.flow.edges.filter((e) => shown[e.kind] && positions.has(e.from) && positions.has(e.to)));

  // Highlights: the scene picked (or under the pointer) lights up its links; a character or
  // a flag lights up scenes.
  let hover: string | null = $state(null);
  let selected: string | null = $state(null);
  let character = $state("");
  let flag = $state("");
  const focus = $derived(hover ?? selected);
  const linked = $derived(
    focus ? new Set([focus, ...edges.filter((e) => e.from === focus || e.to === focus).flatMap((e) => [e.from, e.to])]) : null,
  );
  // The scene picked: where it comes from and where it goes.
  const picked = $derived(selected ? positions.get(selected) : undefined);
  const comesFrom = $derived(selected ? reader.view.flow.edges.filter((e) => e.to === selected && positions.has(e.from)) : []);
  const goesTo = $derived(selected ? reader.view.flow.edges.filter((e) => e.from === selected && positions.has(e.to)) : []);
  const placeName = (key: string) => columns[positions.get(key)?.column ?? -1]?.name ?? "";
  function select(key: string | null) {
    selected = key;
    if (!key || !scroller) return;
    const p = positions.get(key);
    if (!p) return;
    const box = scroller.getBoundingClientRect();
    const inView = p.x > scroller.scrollLeft && p.x + COL < scroller.scrollLeft + box.width && p.y > scroller.scrollTop && p.y < scroller.scrollTop + box.height - 40;
    if (!inView) scroller.scrollTo({ left: p.x - 80, top: p.y - 120, behavior: "smooth" });
  }
  function write(key: string) {
    const label = positions.get(key)?.label;
    if (label) reader.goTo(label.id);
  }
  function play(key: string) {
    const label = positions.get(key)?.label;
    if (!label) return;
    reader.playOpen = true;
    reader.goTo(label.id);
    reader.playFrom = { label: label.id };
  }
  let side: "key" | "doc" = $state("key");
  const speaking = $derived.by(() => {
    if (!character) return null;
    const names = new Set(reader.view.characters.find((c) => c.id === character)?.speakers ?? []);
    const out = new Set<string>();
    for (const [key, { label }] of positions) {
      for (const node of walk(label.body)) {
        const lines = node.type === "line" ? [node] : node.type === "menu" ? node.choices : [];
        if (lines.some((l) => l.speaker && names.has(l.speaker))) {
          out.add(key);
          break;
        }
      }
    }
    return out;
  });
  const flagged = $derived.by(() => {
    if (!flag) return null;
    const out = new Map<string, "set" | "read" | "both">();
    for (const ref of reader.view.script.flags) {
      if (ref.flag !== flag || !ref.label) continue;
      const key = `${ref.span.file}:${ref.label}`;
      const before = out.get(key);
      out.set(key, before && before !== ref.access ? "both" : ref.access);
    }
    return out;
  });

  function path(from: { x: number; y: number }, to: { x: number; y: number }): string {
    const y1 = from.y + NODE_H / 2;
    const y2 = to.y + NODE_H / 2;
    if (Math.abs(from.x - to.x) < 1) {
      // In the same place: an arc on the right.
      const x = from.x + COL;
      const bulge = 30 + Math.min(60, Math.abs(y2 - y1) / 6);
      return `M ${x} ${y1} C ${x + bulge} ${y1}, ${x + bulge} ${y2}, ${x} ${y2}`;
    }
    const forward = to.x > from.x;
    const x1 = forward ? from.x + COL : from.x;
    const x2 = forward ? to.x : to.x + COL;
    const dx = Math.max(40, Math.abs(x2 - x1) / 3) * (forward ? 1 : -1);
    return `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
  }

  const short = (text: string) => (text.length > 26 ? `${text.slice(0, 25)}…` : text);
  const lineCount = (l: Label) => [...walk(l.body)].filter((n) => n.type === "line").length;

  // The walkthrough beside the map: its label links light up and center their label.
  const doc = $derived(reader.view.documents[0]);
  const docHtml = $derived(
    doc
      ? (marked.parse(
          doc.text.replace(/\[\[([^[\]|]+?)\]\]/g, (_, id: string) => `<button type="button" class="lref map-link" data-label="${id.trim()}">${id.trim()}</button>`),
          { async: false, gfm: true },
        ) as string)
      : "",
  );
  let scroller: HTMLDivElement | undefined = $state();
  function focusLabel(id: string) {
    const entry = [...positions.entries()].find(([, p]) => p.label.id === id);
    if (entry) select(entry[0]);
  }
  // Opened from a scene ("Show on map"): point at it.
  $effect(() => {
    const id = reader.mapFocus;
    if (!id || !scroller) return;
    untrack(() => {
      focusLabel(id);
      reader.mapFocus = null;
    });
  });
  function docClick(e: MouseEvent) {
    const target = (e.target as HTMLElement).closest<HTMLElement>("[data-label]");
    if (target?.dataset.label) focusLabel(target.dataset.label);
  }

  // Grids: lines per character and place; flags set and read per place.
  const placeOf = $derived(new Map([...positions].map(([k, p]) => [k, p.column])));
  const people = $derived(
    reader.view.characters
      .map((c) => {
        const names = new Set(c.speakers);
        const counts = columns.map(() => 0);
        for (const [key, { label }] of positions) {
          for (const node of walk(label.body)) {
            if (node.type === "line" && node.speaker && names.has(node.speaker)) counts[placeOf.get(key)!] += 1;
          }
        }
        return { c, counts, total: counts.reduce((a, b) => a + b, 0) };
      })
      .filter((row) => row.total > 0),
  );
  const most = $derived(Math.max(1, ...people.flatMap((r) => r.counts)));
  const flagRows = $derived(
    reader.view.flags
      .map((f) => {
        const cells = columns.map(() => "");
        for (const ref of reader.view.script.flags) {
          if (ref.flag !== f.flag || !ref.label) continue;
          const column = placeOf.get(`${ref.span.file}:${ref.label}`);
          if (column === undefined) continue;
          const mark = ref.access === "set" ? "●" : "○";
          if (!cells[column].includes(mark)) cells[column] = (cells[column] + mark).split("").sort().join("");
        }
        return { f, cells };
      })
      .filter((r) => r.cells.some(Boolean)),
  );
</script>

<div class="map-page gut">
  <div class="map-bar">
    <div class="seg" role="group" aria-label={t("Map")}>
      <button type="button" aria-pressed={view === "flow"} onclick={() => (view = "flow")}>{t("Flow")}</button>
      <button type="button" aria-pressed={view === "people"} onclick={() => (view = "people")}>{t("Characters × places")}</button>
      <button type="button" aria-pressed={view === "flags"} onclick={() => (view = "flags")}>{t("Flags × places")}</button>
    </div>
    {#if view === "flow"}
      <label class="tog">{t("Light up")}
        <select bind:value={character} aria-label={t("Light up a character's scenes")}>
          <option value="">{t("a character…")}</option>
          {#each reader.view.characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
        <select bind:value={flag} aria-label={t("Light up a flag's scenes")}>
          <option value="">{t("a flag…")}</option>
          {#each reader.view.flags as f (f.flag)}<option value={f.flag}>{f.flag}</option>{/each}
        </select>
      </label>
      <span class="hint">{t("Click a scene to see where it comes from and goes to.")}</span>
    {/if}
  </div>

  {#if view === "flow"}
    <div class="map-layout">
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="map-scroll" bind:this={scroller} onclick={(e) => !(e.target as Element).closest(".map-node") && select(null)}>
        <svg {width} {height} role="img" aria-label={t("Scenes by place, and the links between them")}>
          <defs>
            {#each KINDS as k (k.kind)}
              <marker id="arrow-{k.kind}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                <path d="M 0 0 L 10 5 L 0 10 z" class="arrow-head {k.kind}" />
              </marker>
            {/each}
          </defs>
          {#each columns as c, i (c.id)}
            <g class="map-col">
              <rect x={i * (COL + GAP)} y="0" width={COL + 20} height={height} rx="8" class="col-bg" />
              {#if c.group}<text x={i * (COL + GAP) + 10} y="18" class="col-group">{c.group}</text>{/if}
              <text x={i * (COL + GAP) + 10} y="38" class="col-name">{short(c.name)}</text>
            </g>
          {/each}
          {#each edges as e, i (i)}
            {@const from = positions.get(e.from)!}
            {@const to = positions.get(e.to)!}
            <path
              d={path(from, to)}
              class="edge {e.kind}"
              class:dim={focus && e.from !== focus && e.to !== focus}
              class:lit={focus && (e.from === focus || e.to === focus)}
              marker-end="url(#arrow-{e.kind})"
            ><title>{from.label.id} — {kindName(e.kind)} → {to.label.id}{e.flag ? ` (${t("flag {flag}", { flag: e.flag })})` : ""}</title></path>
          {/each}
          {#each [...positions] as [key, p] (key)}
            {@const mark = flagged?.get(key)}
            <g
              class="map-node"
              class:entry={entries.has(key)}
              class:unreached={!reached.has(key)}
              class:dim={(linked && !linked.has(key)) || (speaking && !speaking.has(key)) || (flagged && !mark)}
              class:lit={speaking?.has(key) || !!mark}
              class:pinned={selected === key}
              transform="translate({p.x} {p.y})"
              role="button"
              tabindex="0"
              aria-pressed={selected === key}
              onmouseenter={() => (hover = key)}
              onmouseleave={() => (hover = null)}
              onclick={() => select(selected === key ? null : key)}
              ondblclick={() => write(key)}
              onkeydown={(e) => e.key === "Enter" && select(key)}
            >
              <rect width={COL} height={NODE_H} rx="5" />
              <text x="8" y="14">{short(p.label.id)}</text>
              <text x={COL - 6} y="14" class="count">{mark === "set" ? t("sets") : mark === "read" ? t("reads") : mark === "both" ? t("sets+reads") : lineCount(p.label)}</text>
              <title>{p.label.id}: {plural(lineCount(p.label), "{n} line", "{n} lines")}{entries.has(key) ? `, ${t("started by the game")}` : reached.has(key) ? "" : `, ${t("nothing leads here")}`}</title>
            </g>
          {/each}
        </svg>
      </div>

      <aside class="map-side">
        {#if picked && selected}
          <section class="map-picked">
            <div class="eyebrow">{placeName(selected)}</div>
            <h3>{picked.label.id}</h3>
            <p class="hint">
              {plural(lineCount(picked.label), "{n} line", "{n} lines")}{entries.has(selected) ? ` · ${t("the game starts it by name")}` : reached.has(selected) ? "" : ` · ${t("nothing leads here")}`}
            </p>
            <div class="k">{t("Comes from")}</div>
            {#if comesFrom.length === 0}<p class="hint">{entries.has(selected) ? t("Only the game's code: no scene leads here.") : t("Nothing: no scene leads here, and the game does not start it.")}</p>{/if}
            <ul class="map-links">
              {#each comesFrom as e, i (i)}
                <li><span class="edge-key {e.kind}" title={kindName(e.kind)}></span><button type="button" class="link" onclick={() => select(e.from)}>{positions.get(e.from)?.label.id}</button> <span class="tid">{kindName(e.kind).toLowerCase()}</span></li>
              {/each}
            </ul>
            <div class="k">{t("Goes to")}</div>
            {#if goesTo.length === 0}<p class="hint">{t("Nothing: the conversation ends here.")}</p>{/if}
            <ul class="map-links">
              {#each goesTo as e, i (i)}
                <li><span class="edge-key {e.kind}" title={kindName(e.kind)}></span><span class="tid">{kindName(e.kind).toLowerCase()}</span> <button type="button" class="link" onclick={() => select(e.to)}>{positions.get(e.to)?.label.id}</button>{#if e.flag} <code>{e.flag}</code>{/if}</li>
              {/each}
            </ul>
            <div class="le-row">
              <button type="button" class="btn small primary" onclick={() => write(selected!)}>{t("Write it")}</button>
              {#if reader.writable}<button type="button" class="btn small" onclick={() => play(selected!)}>{t("▶ Play from here")}</button>{/if}
              <button type="button" class="link" onclick={() => select(null)}>{t("Close")}</button>
            </div>
          </section>
        {/if}
        {#if doc}
          <div class="seg" role="group" aria-label={t("Beside the map")}>
            <button type="button" aria-pressed={side === "key"} onclick={() => (side = "key")}>{t("Key")}</button>
            <button type="button" aria-pressed={side === "doc"} onclick={() => (side = "doc")}>{doc.title ?? t("Walkthrough")}</button>
          </div>
        {/if}
        {#if side === "key" || !doc}
          <section class="map-key">
            <h3>{t("Links between scenes")}</h3>
            <p class="hint">{t("{links} links between {labels} scenes. Untick a kind to hide it.", { links: reader.view.flow.edges.length, labels: positions.size })}</p>
            <ul>
              {#each KINDS as k (k.kind)}
                <li>
                  <label class="key-row" class:off={!shown[k.kind]}>
                    <input type="checkbox" bind:checked={shown[k.kind]} />
                    <svg class="key-line" width="44" height="12" aria-hidden="true"><path d="M 2 6 H 34" class="edge {k.kind}" marker-end="url(#arrow-{k.kind})" /></svg>
                    <span class="key-name">{k.name()} <span class="tid">{counts.get(k.kind) ?? 0}</span></span>
                    <span class="key-says">{k.says()}</span>
                  </label>
                </li>
              {/each}
            </ul>
            <h3>{t("Scenes")}</h3>
            <ul class="node-key">
              <li><span class="node-sample entry">scene</span> {t("Dark outline: the game's code starts it by name.")}</li>
              <li><span class="node-sample unreached">scene</span> {t("Red, dashed: nothing leads to it.")}</li>
              <li><span class="node-sample">scene <b>12</b></span> {t("The number is how many lines it has.")}</li>
              <li><span class="node-sample lit">scene</span> {t("Lit up: the character or flag chosen above.")}</li>
            </ul>
          </section>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <section class="map-doc doc" onclick={docClick}>
            <!-- The project's own Markdown; its label links point the map at their scene. -->
            {@html docHtml}
          </section>
        {/if}
      </aside>
    </div>
  {:else if view === "people"}
    <div class="tablewrap grid-wrap">
      <table class="grid">
        <thead><tr><th></th>{#each columns as c (c.id)}<th title={c.name}><span>{c.name}</span></th>{/each}<th>{t("All")}</th></tr></thead>
        <tbody>
          {#each people as row (row.c.id)}
            <tr>
              <th>{row.c.name}</th>
              {#each row.counts as n, i (i)}<td style:--heat={n / most} class:zero={n === 0}>{n || ""}</td>{/each}
              <td class="total">{row.total}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="lede map-legend">{t("Lines each character speaks in each place.")}</p>
  {:else}
    <div class="tablewrap grid-wrap">
      <table class="grid">
        <thead><tr><th></th>{#each columns as c (c.id)}<th title={c.name}><span>{c.name}</span></th>{/each}</tr></thead>
        <tbody>
          {#each flagRows as row (row.f.flag)}
            <tr>
              <th title={row.f.meaning ?? ""}><code>{row.f.flag}</code></th>
              {#each row.cells as cell, i (i)}<td class="flagcell">{cell}</td>{/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="lede map-legend">{t("● set by a label in that place · ○ read by one. Flags set or read only by the game's code are not here.")}</p>
  {/if}
</div>
