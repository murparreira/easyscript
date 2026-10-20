<script lang="ts">
  // A flag, linked to its row in the flags table, its meaning on hover.
  import Hl from "./Hl.svelte";
  import { useReader } from "./reader.svelte";

  let { flag, set = false }: { flag: string; set?: boolean } = $props();
  const reader = useReader();
  const meaning = $derived(reader.view.flags.find((f) => f.flag === flag)?.meaning ?? "");
</script>

<a class="chip flag" class:set href="#f-{flag}" title={meaning}><span class="k">{set ? "sets" : "flag"}</span> <Hl text={flag} /></a>
