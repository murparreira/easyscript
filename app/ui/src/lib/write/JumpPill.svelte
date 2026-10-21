<script lang="ts">
  // Where a jump goes, as something to click: the scene it goes to, or the end.
  import type { Jump } from "../bindings/Jump";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";

  let { jump }: { jump: Jump } = $props();
  const reader = useReader();
  const target = $derived(jump.target);
</script>

{#if target.kind === "label"}
  {#if reader.labelIds.has(target.value)}
    <button type="button" class="pill go" onclick={() => (reader.scene = target.value)} title={t("Open {scene}", { scene: target.value })}>
      {jump.returns ? "↪" : "→"} {target.value}{#if jump.returns}<span class="when"> {t("and back")}</span>{/if}
    </button>
  {:else}
    <span class="pill go broken" title={t("No label with this name")}>→ {target.value}</span>
  {/if}
{:else if target.kind === "end"}
  <span class="pill end">■ {t("End of the scene")}</span>
{:else if target.kind === "end_conversation"}
  <span class="pill end">■ {t("End of the conversation")}</span>
{:else}
  <span class="pill go">→ <code>{"{{"}{target.value}{"}}"}</code></span>
{/if}
