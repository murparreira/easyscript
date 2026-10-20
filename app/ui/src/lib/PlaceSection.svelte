<script lang="ts">
  // A place: its labels, by file and by section.
  import LabelCard from "./LabelCard.svelte";
  import type { PlaceOut } from "./model";
  import Notes from "./Notes.svelte";
  import { useReader } from "./reader.svelte";

  let { place }: { place: PlaceOut } = $props();
  const reader = useReader();
  const visible = $derived(place.files.some((f) => f.sections.some((s) => s.labels.some((l) => reader.shows(l)))));
</script>

{#if visible}
  <section class="place" id="p-{place.id}">
    <div class="ph">
      {#if place.group}<div class="dist">{place.group}</div>{/if}
      <h3>{place.name}</h3>
      {#if reader.showIds}<div class="pmeta">{place.files.map((f) => f.path).join(" · ")}</div>{/if}
    </div>
    {#each place.files as file (file.path)}
      {#each file.sections as section (section.key)}
        {#if section.labels.some((l) => reader.shows(l))}
          <div class="block">
            {#if section.title}<h4>{section.title}</h4>{/if}
            {#if !reader.searching}<Notes notes={section.notes} kind="bnote" />{/if}
            {#each section.labels as label (label.id + label.span.start)}
              {#if reader.shows(label)}<LabelCard {label} />{/if}
            {/each}
          </div>
        {/if}
      {/each}
    {/each}
  </section>
{/if}
