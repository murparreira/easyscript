<script lang="ts">
  // Text with the current search highlighted.
  import { useReader } from "./reader.svelte";
  import { pieces } from "./text";

  let { text }: { text: string } = $props();
  const reader = useReader();
  const parts = $derived(reader.searching ? pieces(text, reader.q) : [{ text, hit: false }]);
</script>

{#each parts as part, i (i)}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}
