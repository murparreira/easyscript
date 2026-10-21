<script lang="ts">
  // Where a jump goes: a label (a link), the end, or an expression.
  import type { Jump } from "./bindings/Jump";
  import Hl from "./Hl.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  let { jump, inline = false }: { jump: Jump; inline?: boolean } = $props();
  const reader = useReader();
  const target = $derived(jump.target);
</script>

<span class="goto" class:inline class:end={target.kind === "end" || target.kind === "end_conversation"}>
  {#if target.kind === "label"}
    {jump.returns ? "↪" : "→"}
    {#if reader.labelIds.has(target.value)}
      <a class="lref" href="#l-{target.value}"><Hl text={target.value} /></a>
    {:else}
      <span class="lref broken" title={t("No label with this name")}><Hl text={target.value} /></span>
    {/if}
    {#if jump.returns}<span class="when">{t("and back")}</span>{/if}
  {:else if target.kind === "end"}
    ■ {t("end")}
  {:else if target.kind === "end_conversation"}
    ■ {t("end of the conversation")}
  {:else}
    → <code class="expr">{"{{"}{target.value}{"}}"}</code>
  {/if}
</span>
