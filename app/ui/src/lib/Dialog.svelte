<script lang="ts">
  // A dialog over the app: a title, a close button, what it says, and its actions at the
  // bottom. Escape and a click outside close it, unless it is busy.
  import { type Snippet, onMount } from "svelte";
  import { t } from "./i18n.svelte";

  let {
    title,
    close,
    busy = false,
    wide = false,
    children,
    actions,
    lede,
  }: { title: string; close: () => void; busy?: boolean; wide?: boolean; children: Snippet; actions?: Snippet; lede?: string } = $props();

  let panel: HTMLDivElement | undefined = $state();
  onMount(() => {
    const first = panel?.querySelector<HTMLElement>("input, textarea, select, button.primary");
    (first ?? panel)?.focus();
  });

  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !busy) {
      e.stopPropagation();
      close();
    }
  }
</script>

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && !busy && close()} onkeydown={keydown}>
  <div class="dialog" class:wide role="dialog" aria-label={title} aria-modal="true" tabindex="-1" bind:this={panel}>
    <header class="dialog-head">
      <h2>{title}</h2>
      <button class="btn" type="button" onclick={close} disabled={busy}>{t("Close")}</button>
    </header>
    {#if lede}<p class="lede">{lede}</p>{/if}
    <div class="dialog-body">{@render children()}</div>
    {#if actions}<footer class="dialog-actions">{@render actions()}</footer>{/if}
  </div>
</div>
