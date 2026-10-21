<script lang="ts">
  // One line of dialogue, to read: face, speaker, text, and what its tags say.
  import type { Line } from "./bindings/Line";
  import type { Mark } from "./bindings/Mark";
  import Face from "./Face.svelte";
  import Hl from "./Hl.svelte";
  import JumpView from "./JumpView.svelte";
  import Marks from "./Marks.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";
  import TagChips from "./TagChips.svelte";
  import TextView from "./TextView.svelte";

  let { line, marks }: { line: Line; marks: Mark[] } = $props();
  const reader = useReader();
  const thought = $derived(reader.shown(line.tags, "thought").length > 0);
  const character = $derived(line.speaker ? reader.characters.get(line.speaker) : undefined);
</script>

<div class="ln" class:thought class:marked={marks.length > 0} class:concurrent={line.concurrent}>
  <span class="avw"><Face image={reader.face(line.speaker, line.tags)} name={line.speaker} /></span>
  <div class="lb">
    <div class="who">
      {#if line.speaker}<span title={character?.name}><Hl text={line.speaker} /></span>{:else}<span class="narration">{t("narration")}</span>{/if}
      <TagChips tags={line.tags} />
      {#if line.concurrent}<span class="tid">{t("at the same time")}</span>{/if}
      <Marks {marks} />
      {#if line.id && reader.showIds}<span class="tid">{line.id}</span>{/if}
    </div>
    <div class="say"><TextView segments={line.text} /></div>
    {#if line.jump}<JumpView jump={line.jump} inline />{/if}
  </div>
</div>
