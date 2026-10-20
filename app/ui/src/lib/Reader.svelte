<script lang="ts">
  // The read view: the whole script, searchable, with its documents and indexes.
  import { type Snippet, onMount, tick, untrack } from "svelte";
  import type { ReadView } from "./bindings/ReadView";
  import DocumentSection from "./DocumentSection.svelte";
  import FlagsTable from "./FlagsTable.svelte";
  import PlaceSection from "./PlaceSection.svelte";
  import ProblemsSection from "./ProblemsSection.svelte";
  import { Reader, provideReader, type VariantChoice } from "./reader.svelte";
  import StatsSection from "./StatsSection.svelte";
  import Toc from "./Toc.svelte";

  let { view, actions }: { view: ReadView; actions?: Snippet } = $props();
  // Later views (a reload) go in through the effect below; the reader keeps its state.
  const reader = new Reader(untrack(() => view));
  provideReader(reader);
  $effect(() => {
    reader.view = view;
  });

  // Each face once, as a class, instead of a data URI on every line. A constructed
  // stylesheet, because the app's content security policy refuses `<style>` elements.
  const faces = new CSSStyleSheet();
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, faces];
  $effect(() => {
    faces.replaceSync(reader.imageCss);
  });

  // The page is drawn after it loads, too late for the browser to find `#l-label` itself.
  onMount(() => {
    tick().then(() => {
      const target = decodeURIComponent(location.hash.slice(1));
      if (target) document.getElementById(target)?.scrollIntoView({ block: "start", behavior: "instant" });
    });
  });

  let typed = $state("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  function search() {
    clearTimeout(timer);
    timer = setTimeout(() => (reader.query = typed), 150);
  }

  const shown = $derived(reader.allLabels().filter((l) => reader.shows(l)).length);
  const speakers = $derived(reader.view.stats.speakers.filter((s) => s.name).length);
  const errors = $derived(reader.view.problems.filter((p) => p.severity === "error").length);

  function choose(variant: string, choice: VariantChoice) {
    reader.variants = { ...reader.variants, [variant]: choice };
  }
</script>

<header class="masthead gut">
  <div class="in">
    <div class="eyebrow">The whole script</div>
    <h1>{reader.view.title}</h1>
    <div class="meta-line">
      {reader.view.stats.labels} labels · {reader.view.stats.lines} lines · {reader.view.stats.choices} choices · {speakers} speakers
      {#if reader.view.problems.length > 0}
        · <a href="#problems" class:errors={errors > 0}>{reader.view.problems.length} problems</a>
      {/if}
    </div>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
</header>

<div class="bar gut">
  <div class="in">
    <label class="search">
      <input type="search" placeholder="Search lines, speakers, labels, flags…" autocomplete="off" aria-label="Search the script" bind:value={typed} oninput={search} />
    </label>
    {#each reader.view.variants as variant (variant.name)}
      {@const choice = reader.variants[variant.name] ?? "all"}
      <div class="seg" role="group" aria-label={variant.name}>
        <button type="button" aria-pressed={choice === "all"} onclick={() => choose(variant.name, "all")}>All</button>
        <button type="button" aria-pressed={choice === "true"} onclick={() => choose(variant.name, "true")}>{variant.if_true}</button>
        <button type="button" aria-pressed={choice === "false"} onclick={() => choose(variant.name, "false")}>{variant.if_false}</button>
      </div>
    {/each}
    <label class="tog"><input type="checkbox" bind:checked={reader.showIds} /> Ids and file names</label>
    <label class="tog"><input type="checkbox" bind:checked={reader.showMarks} /> Highlight marked lines</label>
    {#if reader.searching}
      <div class="status">{shown} {shown === 1 ? "label matches" : "labels match"}. Clear the search to see everything.</div>
    {/if}
  </div>
</div>

<div class="layout gut" class:no-ids={!reader.showIds} class:show-marks={reader.showMarks}>
  <Toc />
  <main>
    {#if !reader.searching}
      {#each reader.view.documents as doc, i (doc.path)}<DocumentSection {doc} index={i} />{/each}
    {/if}
    <section class="sec" id="script">
      {#each reader.groups as group, i (i)}
        {#each group.places as place (place.id)}<PlaceSection {place} />{/each}
      {/each}
    </section>
    {#if !reader.searching}
      {#if reader.view.flags.length > 0}<FlagsTable />{/if}
      <StatsSection />
      <ProblemsSection />
    {/if}
  </main>
</div>
