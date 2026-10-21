<script lang="ts">
  // Commits as a writer reads them: who, when, what they said, and the scenes they changed
  // (a click opens one to write).
  import type { CommitInfo } from "./bindings/CommitInfo";
  import type { SceneRef } from "./bindings/SceneRef";
  import { t } from "./i18n.svelte";
  import { ago, sceneName } from "./team.svelte";

  let { commits, open, limit = 8 }: { commits: CommitInfo[]; open: (scene: SceneRef) => void; limit?: number } = $props();
  let all = $state(false);
  const shown = $derived(all ? commits : commits.slice(0, limit));
</script>

<ul class="commits">
  {#each shown as c (c.id)}
    <li>
      <div class="commit-head">
        <span class="who">{c.mine ? t("You") : c.author}</span>
        <span class="when" title={new Date(c.time * 1000).toLocaleString()}>{ago(c.time)}</span>
        <span class="tid">{c.id.slice(0, 7)}</span>
      </div>
      <div class="summary">{c.summary}</div>
      {#if c.scenes.length > 0}
        <div class="scene-chips">
          {#each c.scenes as s (s.path + "#" + s.label)}
            <button type="button" class="chip scene-chip" onclick={() => open(s)} title={s.path}>{sceneName(s)}</button>
          {/each}
        </div>
      {/if}
    </li>
  {/each}
</ul>
{#if commits.length > shown.length}
  <button type="button" class="link" onclick={() => (all = true)}>{t("Show all {n}", { n: commits.length })}</button>
{/if}
