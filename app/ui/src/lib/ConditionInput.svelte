<script lang="ts">
  // A condition, with the project's flags at hand.
  import { useReader } from "./reader.svelte";

  let { value = $bindable(), label = "Condition" }: { value: string; label?: string } = $props();
  const reader = useReader();
  const known = $derived(new Set(reader.view.flags.map((f) => f.flag)));
  const call = $derived(reader.flagCall);

  // Flags the condition names that nothing sets and the project file does not describe.
  const unknown = $derived.by(() => {
    if (!call) return [];
    const [before, after] = call.split("{flag}");
    const names: string[] = [];
    let at = value.indexOf(before);
    while (at >= 0) {
      const start = at + before.length;
      const end = value.indexOf(after, start);
      if (end < 0) break;
      names.push(value.slice(start, end));
      at = value.indexOf(before, end);
    }
    return names.filter((n) => !known.has(n));
  });

  function add(flag: string) {
    if (!call || !flag) return;
    const check = call.replace("{flag}", flag);
    value = value.trim() ? `${value.trim()} and ${check}` : check;
  }
</script>

<div class="condition-input">
  <label class="condition-field"><span class="k">{label}</span>
    <input type="text" bind:value spellcheck="false" placeholder="always" /></label>
  {#if call && reader.view.flags.length > 0}
    <select aria-label="Add a flag" onchange={(e) => { add(e.currentTarget.value); e.currentTarget.value = ""; }}>
      <option value="">Add a flag…</option>
      {#each reader.view.flags as flag (flag.flag)}
        <option value={flag.flag}>{flag.flag}{flag.meaning ? ` — ${flag.meaning.slice(0, 60)}` : ""}</option>
      {/each}
    </select>
  {/if}
  {#if unknown.length > 0}<p class="warn">Not a known flag: {unknown.join(", ")}</p>{/if}
</div>
