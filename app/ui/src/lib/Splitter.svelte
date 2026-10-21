<script lang="ts">
  // A handle on a panel's edge to make it wider or narrower by dragging (or with the arrow
  // keys); a double click puts it back.
  import { t } from "./i18n.svelte";

  let {
    width = $bindable(),
    edge,
    min,
    max,
    initial,
  }: { width: number; edge: "left" | "right"; min: number; max: number; initial: number } = $props();

  const clamp = (w: number) => Math.round(Math.min(max, Math.max(min, w)));
  let start: { x: number; width: number } | null = null;

  function down(e: PointerEvent) {
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    start = { x: e.clientX, width };
  }
  function move(e: PointerEvent) {
    if (!start) return;
    const dx = e.clientX - start.x;
    width = clamp(start.width + (edge === "right" ? dx : -dx));
  }
  function up() {
    start = null;
  }
  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 48 : 16;
    const grow = edge === "right" ? "ArrowRight" : "ArrowLeft";
    const shrink = edge === "right" ? "ArrowLeft" : "ArrowRight";
    if (e.key === grow) width = clamp(width + step);
    else if (e.key === shrink) width = clamp(width - step);
    else return;
    e.preventDefault();
  }
</script>

<!-- A focusable separator is the ARIA window splitter, which the linter does not know. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter {edge}"
  role="separator"
  aria-orientation="vertical"
  aria-valuenow={width}
  aria-valuemin={min}
  aria-valuemax={max}
  aria-label={t("Drag to resize; double-click to reset")}
  title={t("Drag to resize; double-click to reset")}
  tabindex="0"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  ondblclick={() => (width = initial)}
  onkeydown={key}
></div>
