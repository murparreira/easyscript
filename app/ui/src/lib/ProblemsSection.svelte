<script lang="ts">
  // What `dialogue-maker check` reports, where it is.
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  const reader = useReader();
</script>

{#if reader.view.problems.length > 0}
  <section class="sec" id="problems">
    <h2>{t("Problems")}</h2>
    <p class="lede">{t("What a check of the files found. Fix them in the files; this page follows.")}</p>
    <ul class="problems">
      {#each reader.view.problems as p, i (i)}
        <li class={p.severity}>
          <span class="sev">{p.severity === "error" ? t("error") : t("warning")}</span>
          {#if p.location}<code class="loc">{p.location.path}:{p.location.line}</code>{/if}
          <span>{p.message}</span>
          <span class="tid">{p.code}</span>
        </li>
      {/each}
    </ul>
  </section>
{/if}
