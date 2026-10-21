<script lang="ts">
  // The project's variants (a mode, a difficulty) as filters: everything, or one side.
  import { t } from "./i18n.svelte";
  import { type VariantChoice, useReader } from "./reader.svelte";

  const reader = useReader();
  function choose(variant: string, choice: VariantChoice) {
    reader.variants = { ...reader.variants, [variant]: choice };
  }
</script>

{#each reader.view.variants as variant (variant.name)}
  {@const choice = reader.variants[variant.name] ?? "all"}
  <div class="seg" role="group" aria-label={variant.name}>
    <button type="button" aria-pressed={choice === "all"} onclick={() => choose(variant.name, "all")}>{t("All")}</button>
    <button type="button" aria-pressed={choice === "true"} onclick={() => choose(variant.name, "true")}>{variant.if_true}</button>
    <button type="button" aria-pressed={choice === "false"} onclick={() => choose(variant.name, "false")}>{variant.if_false}</button>
  </div>
{/each}
