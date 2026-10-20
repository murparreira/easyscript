<script lang="ts">
  // A speaker's face, or their initials when there is none.
  import { initials } from "./text";

  let { image, name, size = "" }: { image: string | null; name: string | null; size?: string } = $props();
</script>

{#if image}
  <span class="av face {image} {size}" role="img" aria-label={name ?? ""}></span>
{:else}
  <span class="av mono-av {size}" aria-hidden="true">{initials(name ?? "·")}</span>
{/if}
