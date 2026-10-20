<script lang="ts">
  // The tags of a line the way the project file says to show them.
  import type { Tag } from "./bindings/Tag";
  import { useReader } from "./reader.svelte";

  let { tags }: { tags: Tag[] } = $props();
  const reader = useReader();
</script>

{#each tags as tag, i (i)}{@const show = reader.tagSpec(tag.name)?.show}{#if show === "thought"}<span
      class="ttag">thinks</span
    >{:else if show === "shake"}<span class="shtag" title="Shakes the screen">shake</span>{:else if show === "voice"}<span
      class="vtag v-{tag.value}"
      title="Voice: {tag.value}">{tag.value}</span
    >{:else if show === "face"}{#if reader.showIds}<span class="tid">{tag.value}</span>{/if}{:else if reader.showIds}<span
      class="tid">{tag.name}{tag.value ? `=${tag.value}` : ""}</span
    >{/if}{/each}
