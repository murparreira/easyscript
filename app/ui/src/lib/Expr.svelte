<script lang="ts">
  // An expression, with flag calls shown as chips that link to the flags table.
  import FlagChip from "./FlagChip.svelte";
  import Hl from "./Hl.svelte";
  import { exprPieces } from "./model";
  import { useReader } from "./reader.svelte";

  let { expr }: { expr: string } = $props();
  const reader = useReader();
  const parts = $derived(exprPieces(expr, reader.view.flag_functions));
</script>

{#each parts as part, i (i)}{#if "flag" in part}<FlagChip flag={part.flag} set={part.access === "set"} />{:else if part.text.trim()}<code
      class="expr"><Hl text={part.text.trim()} /></code
    >{/if}{/each}
