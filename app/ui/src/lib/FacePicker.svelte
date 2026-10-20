<script lang="ts">
  // Pick a face from the files a face tag can name; the speaker's usual faces first.
  import { useReader } from "./reader.svelte";

  let {
    tag,
    speaker,
    value = $bindable(),
  }: { tag: string; speaker: string | null; value: string | null } = $props();
  const reader = useReader();
  const spec = $derived(reader.tagSpec(tag));
  let filter = $state("");

  const faces = $derived.by(() => {
    const usual = reader.facesBySpeaker.get(speaker ?? "") ?? [];
    const all = spec?.available ?? [];
    const rest = all.filter((f) => !usual.includes(f));
    const ordered = [...usual.filter((f) => all.includes(f)), ...rest];
    const q = filter.trim().toLowerCase();
    return q ? ordered.filter((f) => f.toLowerCase().includes(q)) : ordered;
  });
  const image = (face: string) => (spec?.path ? reader.imageClasses.get(spec.path.replace("{value}", face)) : undefined);
</script>

<div class="face-picker">
  <div class="face-picker-head">
    <span class="k">Face</span>
    <span class="tid">{value ?? "none"}</span>
    <input type="search" placeholder="Filter faces…" bind:value={filter} aria-label="Filter faces" />
    <button type="button" class="link" onclick={() => (value = null)}>none</button>
  </div>
  <div class="face-grid" role="listbox" aria-label="Faces">
    {#each faces as face (face)}
      <button
        type="button"
        class="face-option"
        class:chosen={face === value}
        title={face}
        role="option"
        aria-selected={face === value}
        onclick={() => (value = face)}
      >
        <span class="av face {image(face) ?? ''}"></span>
      </button>
    {/each}
    {#if faces.length === 0}<p class="nothing">No faces match.</p>{/if}
  </div>
</div>
