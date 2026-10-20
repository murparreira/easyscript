<script lang="ts">
  // The read view: the whole script, searchable, with its documents and indexes.
  import { type Snippet, onMount, tick, untrack } from "svelte";
  import type { ReadView } from "./bindings/ReadView";
  import DocumentSection from "./DocumentSection.svelte";
  import EditBar from "./EditBar.svelte";
  import { type Editor, provideEditor } from "./editor.svelte";
  import FlagsTable from "./FlagsTable.svelte";
  import PlaceSection from "./PlaceSection.svelte";
  import ProjectPanel from "./ProjectPanel.svelte";
  import StoryMap from "./StoryMap.svelte";
  import ProblemsSection from "./ProblemsSection.svelte";
  import { Reader, provideReader, type VariantChoice } from "./reader.svelte";
  import StatsSection from "./StatsSection.svelte";
  import Toc from "./Toc.svelte";

  let { view, actions, editor }: { view: ReadView; actions?: Snippet; editor?: Editor } = $props();
  // The editor exists in the desktop app only; the static page reads.
  provideEditor(untrack(() => editor));
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

  // Undo, redo and save from the keyboard; text fields keep their own undo.
  function keydown(e: KeyboardEvent) {
    if (!editor || !(e.ctrlKey || e.metaKey)) return;
    const typing = e.target instanceof HTMLElement && e.target.closest("input, textarea, select");
    const key = e.key.toLowerCase();
    if (key === "s") {
      e.preventDefault();
      if (editor.state.changed.length > 0) editor.save();
    } else if (!typing && key === "z") {
      e.preventDefault();
      if (e.shiftKey) editor.redo();
      else editor.undo();
    } else if (!typing && key === "y") {
      e.preventDefault();
      editor.redo();
    }
  }

  function choose(variant: string, choice: VariantChoice) {
    reader.variants = { ...reader.variants, [variant]: choice };
  }
</script>

<svelte:window onkeydown={keydown} />

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
    <div class="seg" role="group" aria-label="Look at">
      <button type="button" aria-pressed={reader.tab === "script"} onclick={() => (reader.tab = "script")}>Script</button>
      <button type="button" aria-pressed={reader.tab === "map"} onclick={() => (reader.tab = "map")}>Story map</button>
      {#if editor}<button type="button" aria-pressed={reader.tab === "project"} onclick={() => (reader.tab = "project")}>Project</button>{/if}
    </div>
    {#if editor && reader.tab === "script"}<label class="tog edit-toggle"><input type="checkbox" bind:checked={editor.on} /> Edit</label>{/if}
    <label class="tog"><input type="checkbox" bind:checked={reader.showIds} /> Ids and file names</label>
    <label class="tog"><input type="checkbox" bind:checked={reader.showMarks} /> Highlight marked lines</label>
    {#if reader.searching}
      <div class="status">{shown} {shown === 1 ? "label matches" : "labels match"}. Clear the search to see everything.</div>
    {/if}
  </div>
</div>

{#if editor && reader.tab === "project"}
  <div class="layout single gut editing"><main><ProjectPanel /></main></div>
{:else if reader.tab === "map"}
  <StoryMap />
{:else}
<div class="layout gut" class:no-ids={!reader.showIds} class:show-marks={reader.showMarks} class:editing={editor?.on}>
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

{/if}

{#if editor}<EditBar />{/if}
