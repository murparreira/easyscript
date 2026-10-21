<script lang="ts">
  // Who talks, and how much.
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  const reader = useReader();
  const stats = $derived(reader.view.stats);
  const most = $derived(Math.max(1, ...stats.speakers.map((s) => s.count)));
  const marked = $derived(stats.marks.reduce((n, m) => n + m.count, 0));
</script>

<section class="sec" id="stats">
  <h2>{t("Who talks")}</h2>
  <p class="lede">
    {t("Lines per speaker, choices left out.")}
    {t("{lines} lines, {choices} choices in {menus} menus, {words} words.", { lines: stats.lines, choices: stats.choices, menus: stats.menus, words: stats.words })}
    {#if marked > 0}{t("{n} marks: {list}.", { n: marked, list: stats.marks.map((m) => `${m.count} ${m.name}`).join(", ") })}{/if}
  </p>
  <ul class="bars">
    {#each stats.speakers as row (row.name)}
      <li>
        <span class="sn">{row.name || t("narration")}</span>
        <span class="track"><i style:width="{(100 * row.count) / most}%"></i></span>
        <span class="sv">{row.count}</span>
      </li>
    {/each}
  </ul>
</section>
