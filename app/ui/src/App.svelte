<script lang="ts">
  // The app shell. In the desktop app: open a project folder, and follow its files as they
  // change. As a static page (`dialogue-maker build`): read the view embedded in the page.
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import type { ReadView } from "./lib/bindings/ReadView";
  import Reader from "./lib/Reader.svelte";

  const LAST = "dialogue-maker:last-project";

  let view: ReadView | null = $state.raw(embedded());
  let path: string | null = $state(null);
  let error: string | null = $state(null);
  let loading = $state(false);
  const desktop = isTauri();

  function embedded(): ReadView | null {
    const data = document.getElementById("dm-view")?.textContent?.trim();
    return data ? (JSON.parse(data) as ReadView) : null;
  }

  function remember(value: string | null) {
    try {
      if (value) localStorage.setItem(LAST, value);
      else localStorage.removeItem(LAST);
    } catch {
      // No storage: the app just won't reopen the last project.
    }
  }

  async function openProject(folder: string) {
    loading = true;
    error = null;
    try {
      view = await invoke<ReadView>("open_project", { path: folder });
      path = folder;
      remember(folder);
    } catch (e) {
      error = `${folder}: ${e}`;
      remember(null);
    } finally {
      loading = false;
    }
  }

  async function choose() {
    const folder = await open({ directory: true, title: "Open a game project" });
    if (typeof folder === "string") await openProject(folder);
  }

  onMount(() => {
    if (view) return;
    if (!desktop) {
      if (import.meta.env.DEV) {
        // `dialogue-maker build --json -o app/ui/dev/view.json` for working on the view in a
        // browser; a `dm-dev-reload` event reads it again, as a change on disk would.
        const load = (file = "view.json") =>
          fetch(`/dev/${file}?${Date.now()}`)
            .then((r) => (r.ok ? r.json() : null))
            .then((v) => (view = v));
        load();
        window.addEventListener("dm-dev-reload", (e) => load((e as CustomEvent<string | undefined>).detail));
      }
      return;
    }
    let last: string | null = null;
    try {
      last = localStorage.getItem(LAST);
    } catch {
      last = null;
    }
    // A folder named on the command line comes first, then the last one opened.
    invoke<string | null>("startup_project").then((given) => {
      const folder = given ?? last;
      if (folder) openProject(folder);
    });
    const stop = listen("project-changed", async () => {
      try {
        view = await invoke<ReadView>("reload");
      } catch (e) {
        error = String(e);
      }
    });
    return () => {
      stop.then((f) => f());
    };
  });
</script>

{#if view}
  <Reader {view}>
    {#snippet actions()}
      {#if desktop}
        <button class="btn" type="button" onclick={choose}>Open another project…</button>
        {#if path}<span class="pmeta">{path} · follows changes on disk</span>{/if}
        {#if error}<span class="error">{error}</span>{/if}
      {/if}
    {/snippet}
  </Reader>
{:else}
  <main class="welcome gut">
    <h1>dialogue-maker</h1>
    {#if desktop}
      <p>Open a game project's folder to read its script. Nothing is changed on disk.</p>
      <button class="btn primary" type="button" onclick={choose} disabled={loading}>{loading ? "Opening…" : "Open a project…"}</button>
      {#if error}<p class="error">{error}</p>{/if}
    {:else}
      <p>No script in this page.</p>
    {/if}
  </main>
{/if}
