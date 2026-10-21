<script lang="ts">
  // The contents rail: documents, places by group, the indexes.
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  const reader = useReader();
  let open = $state(typeof window === "undefined" || !window.matchMedia("(max-width: 900px)").matches);
</script>

<nav class="toc" aria-label={t("Contents")}>
  <details bind:open>
    <summary>{t("Contents")}</summary>
    <ol>
      {#each reader.view.documents as doc, i (doc.path)}<li><a href="#d-{i}">{doc.title}</a></li>{/each}
      {#each reader.groups as group, i (i)}
        {#if group.name}<li class="dh">{group.name}</li>{/if}
        {#each group.places as place (place.id)}<li><a href="#p-{place.id}">{place.name}</a></li>{/each}
      {/each}
      <li class="dh">{t("Indexes")}</li>
      {#if reader.view.flags.length > 0}<li><a href="#flags">{t("Flags")}</a></li>{/if}
      <li><a href="#stats">{t("Who talks")}</a></li>
      {#if reader.view.problems.length > 0}<li><a href="#problems">{t("Problems ({n})", { n: reader.view.problems.length })}</a></li>{/if}
    </ol>
  </details>
</nav>
