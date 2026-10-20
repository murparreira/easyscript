<script lang="ts">
  // Comments written above something in the script. Writers wrap comments by hand, so
  // lines run together into paragraphs; an empty comment line starts a new one.
  import type { Note } from "./bindings/Note";
  import Hl from "./Hl.svelte";

  let { notes, kind = "nnote" }: { notes: Note[]; kind?: string } = $props();
  const paragraphs = $derived(
    notes
      .map((n) => n.text)
      .join("\n")
      .split(/\n\s*\n/)
      .map((p) => p.replace(/\s*\n\s*/g, " ").trim())
      .filter(Boolean),
  );
</script>

{#if paragraphs.length > 0}
  <div class={kind}>
    {#each paragraphs as text, i (i)}<p><Hl {text} /></p>{/each}
  </div>
{/if}
