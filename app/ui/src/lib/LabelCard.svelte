<script lang="ts">
  // A label: its name, what was written about it, and its steps.
  import type { Label } from "./bindings/Label";
  import Hl from "./Hl.svelte";
  import Marks from "./Marks.svelte";
  import Nodes from "./Nodes.svelte";
  import Notes from "./Notes.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  let { label }: { label: Label } = $props();
  const reader = useReader();
</script>

<article class="label" id="l-{label.id}">
  <div class="lh">
    <a class="lid" href="#l-{label.id}"><Hl text={label.id} /></a>
    <Marks marks={label.marks} />
    {#if reader.writable}
      <button type="button" class="link write-it" onclick={() => { reader.tab = "write"; reader.scene = label.id; }}>{t("Write")}</button>
    {/if}
  </div>
  <Notes notes={label.notes} kind="lnote" />
  <Nodes nodes={label.body} />
</article>
