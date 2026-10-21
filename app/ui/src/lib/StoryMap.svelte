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

  // Which kinds of edges to draw.
  const KINDS: Array<{ kind: FlowKind; name: () => string }> = [
    { kind: "jump", name: () => t("jumps") },
    { kind: "choice", name: () => t("choices") },
    { kind: "conditional", name: () => t("conditions") },
    { kind: "return", name: () => t("there and back") },
    { kind: "fall_through", name: () => t("falling through") },
    { kind: "flag", name: () => t("flags") },
  ];
  let shown: Record<string, boolean> = $state(Object.fromEntries(KINDS.map((k) => [k.kind, true])));
  const edges = $derived(reader.view.flow.edges.filter((e) => shown[e.kind] && positions.has(e.from) && positions.has(e.to)));

  // Highlights: hovering a label lights up its links; a character or a flag lights up labels.
  let hover: string | null = $state(null);
  let character = $state("");
  let flag = $state("");
  const linked = $derived(
    hover ? new Set(edges.filter((e) => e.from === hover || e.to === hover).flatMap((e) => [e.from, e.to])) : null,
  );
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
    if (!entry || !scroller) return;
    hover = entry[0];
    scroller.scrollTo({ left: entry[1].x - 80, top: entry[1].y - 120, behavior: "smooth" });
  }
  // Opened from a scene ("Show on map"): point at it.
  $effect(() => {
    const id = reader.mapFocus;
    if (!id || !scroller) return;
    untrack(() => {
      focusLabel(id);
      pinned = id;
      reader.mapFocus = null;
    });
  });
  let pinned: string | null = $state(null);
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
      <label class="tog">{t("Highlight")}
        <select bind:value={character} aria-label={t("Highlight a character's labels")}>
          <option value="">{t("no character")}</option>
          {#each reader.view.characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
        <select bind:value={flag} aria-label={t("Highlight a flag's labels")}>
          <option value="">{t("no flag")}</option>
          {#each reader.view.flags as f (f.flag)}<option value={f.flag}>{f.flag}</option>{/each}
        </select>
      </label>
      <span class="kinds">
        {#each KINDS as k (k.kind)}<label class="tog"><input type="checkbox" bind:checked={shown[k.kind]} /> <span class="edge-key {k.kind}"></span>{k.name()}</label>{/each}
      </span>
    {/if}
  </div>

  {#if view === "flow"}
    <div class="map-layout" class:with-doc={!!doc}>
      <div class="map-scroll" bind:this={scroller}>
        <svg {width} {height} role="img" aria-label={t("Labels by place, and the links between them")}>
          <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M 0 0 L 10 5 L 0 10 z" class="arrow-head" />
            </marker>
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
              class:dim={linked && !(linked.has(e.from) && linked.has(e.to))}
              marker-end="url(#arrow)"
            ><title>{from.label.id} → {to.label.id}{e.flag ? ` (${t("flag {flag}", { flag: e.flag })})` : ""}</title></path>
          {/each}
          {#each [...positions] as [key, p] (key)}
            {@const mark = flagged?.get(key)}
            <g
              class="map-node"
              class:entry={entries.has(key)}
              class:unreached={!reached.has(key)}
              class:dim={(linked && !linked.has(key)) || (speaking && !speaking.has(key)) || (flagged && !mark)}
              class:lit={speaking?.has(key) || !!mark}
              class:pinned={pinned === p.label.id}
              transform="translate({p.x} {p.y})"
              role="button"
              tabindex="0"
              onmouseenter={() => (hover = key)}
              onmouseleave={() => (hover = null)}
              onfocus={() => (hover = key)}
              onblur={() => (hover = null)}
              onclick={() => reader.goTo(p.label.id)}
              onkeydown={(e) => e.key === "Enter" && reader.goTo(p.label.id)}
            >
              <rect width={COL} height={NODE_H} rx="5" />
              <text x="8" y="14">{short(p.label.id)}</text>
              <text x={COL - 6} y="14" class="count">{mark === "set" ? t("sets") : mark === "read" ? t("reads") : mark === "both" ? t("sets+reads") : lineCount(p.label)}</text>
              <title>{p.label.id}: {plural(lineCount(p.label), "{n} line", "{n} lines")}{entries.has(key) ? `, ${t("started by the game")}` : reached.has(key) ? "" : `, ${t("nothing leads here")}`}</title>
            </g>
          {/each}
        </svg>
      </div>
      {#if doc}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
        <aside class="map-doc doc" onclick={docClick}>
          <!-- The project's own Markdown; its label links center the map on their label. -->
          {@html docHtml}
        </aside>
      {/if}
    </div>
    <p class="lede map-legend">{t("A dark outline: the game starts the label by name; dashed: nothing leads to it. The number is its lines.")} {t("{links} links between {labels} labels; hover a label for its links, click it to read it.", { links: reader.view.flow.edges.length, labels: positions.size })}</p>
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
