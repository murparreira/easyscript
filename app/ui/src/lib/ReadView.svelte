<script lang="ts">
  // The whole script to read, as one document: its documents, the places and their labels,
  // the indexes. Searchable, with the project's variant filters.
  import { onMount, tick } from "svelte";
  import DocumentSection from "./DocumentSection.svelte";
  import FlagsTable from "./FlagsTable.svelte";
  import { plural, t } from "./i18n.svelte";
  import PlaceSection from "./PlaceSection.svelte";
  import ProblemsSection from "./ProblemsSection.svelte";
  import { useReader } from "./reader.svelte";
  import StatsSection from "./StatsSection.svelte";
  import Toc from "./Toc.svelte";
  import VariantFilter from "./VariantFilter.svelte";

  const reader = useReader();

  // The page is drawn after it loads, too late for the browser to find `#l-label` itself.
  onMount(() => {
    tick().then(() => {
      const target = decodeURIComponent(location.hash.slice(1));
      if (target) document.getElementById(target)?.scrollIntoView({ block: "start", behavior: "instant" });
    });
  });

  let typed = $state(reader.query);
  let timer: ReturnType<typeof setTimeout> | undefined;
  function search() {
    clearTimeout(timer);
    timer = setTimeout(() => (reader.query = typed), 150);
  }

  const shown = $derived(reader.allLabels().filter((l) => reader.shows(l)).length);
  const speakers = $derived(reader.view.stats.speakers.filter((s) => s.name).length);
  const errors = $derived(reader.view.problems.filter((p) => p.severity === "error").length);
</script>

<div class="subbar">
  <input class="search" type="search" placeholder={t("Search lines, speakers, labels, flags…")} autocomplete="off" aria-label={t("Search the script")} bind:value={typed} oninput={search} />
  <VariantFilter />
  <span class="meta-line">
    {#if reader.searching}
      {plural(shown, "{n} label matches.", "{n} labels match.")}
    {:else}
      {plural(reader.view.stats.labels, "{n} label", "{n} labels")} · {plural(reader.view.stats.lines, "{n} line", "{n} lines")} · {plural(reader.view.stats.choices, "{n} choice", "{n} choices")} · {plural(speakers, "{n} speaker", "{n} speakers")}
      {#if reader.view.problems.length > 0}
        · <a href="#problems" class:errors={errors > 0}>{plural(reader.view.problems.length, "{n} problem", "{n} problems")}</a>
      {/if}
    {/if}
  </span>
</div>

<div class="layout" class:no-ids={!reader.showIds} class:show-marks={reader.showMarks}>
  <Toc />
  <article class="read-main">
    <h1 class="doc-title">{reader.view.title}</h1>
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
  </article>
</div>
