<script lang="ts">
  // Authorship marks: who wrote this, and why.
  import type { Mark } from "./bindings/Mark";

  let { marks }: { marks: Mark[] } = $props();
</script>

{#each marks as mark, i (i)}<span class="mark" title={mark.note ? `${mark.who}: ${mark.note}` : mark.who}
    >✏ {mark.who}{#if mark.note}<span class="mark-note">{` · ${mark.note}`}</span>{/if}</span
  >{/each}
