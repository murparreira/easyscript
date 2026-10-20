<script lang="ts">
  // A label: its name, what was written about it, and its steps.
  import type { Label } from "./bindings/Label";
  import Hl from "./Hl.svelte";
  import Marks from "./Marks.svelte";
  import Nodes from "./Nodes.svelte";
  import Notes from "./Notes.svelte";

  let { label }: { label: Label } = $props();
</script>

<article class="label" id="l-{label.id}">
  <div class="lh">
    <a class="lid" href="#l-{label.id}"><Hl text={label.id} /></a>
    <Marks marks={label.marks} />
  </div>
  <Notes notes={label.notes} kind="lnote" />
  <Nodes nodes={label.body} />
</article>
