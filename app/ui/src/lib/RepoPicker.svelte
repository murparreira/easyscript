<script lang="ts">
  // The signed-in account's repositories, to pick one: searchable, last pushed first.
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { RepoInfo } from "./bindings/RepoInfo";
  import { t } from "./i18n.svelte";

  let { chosen = $bindable() }: { chosen: RepoInfo | null } = $props();
  let repos = $state<RepoInfo[] | null>(null);
  let error: string | null = $state(null);
  let query = $state("");
  const shown = $derived(
    (repos ?? []).filter((r) => {
      const q = query.trim().toLowerCase();
      return !q || `${r.owner}/${r.name} ${r.description ?? ""}`.toLowerCase().includes(q);
    }),
  );

  onMount(async () => {
    try {
      repos = await invoke<RepoInfo[]>("github_repos");
    } catch (e) {
      error = String(e);
    }
  });
</script>

<div class="repo-picker">
  <input type="search" placeholder={t("Find a repository…")} bind:value={query} aria-label={t("Find a repository")} />
  {#if error}
    <p class="error">{error}</p>
  {:else if repos === null}
    <p class="hint">{t("Asking GitHub for your repositories…")}</p>
  {:else if shown.length === 0}
    <p class="hint">{t("No repository matches.")}</p>
  {:else}
    <ul class="repo-list" role="listbox" aria-label={t("Repositories")}>
      {#each shown as repo (repo.owner + "/" + repo.name)}
        {@const on = chosen?.owner === repo.owner && chosen?.name === repo.name}
        <li>
          <button type="button" role="option" aria-selected={on} class:on onclick={() => (chosen = repo)}>
            <span class="repo-name">{repo.owner}/<b>{repo.name}</b></span>
            {#if repo.private}<span class="chip quiet">{t("private")}</span>{/if}
            {#if repo.description}<span class="hint">{repo.description}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
