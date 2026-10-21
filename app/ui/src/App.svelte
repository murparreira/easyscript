<script lang="ts">
  // The app shell. In the desktop app: open a project folder, and follow its files as they
  // change. As a static page (`dialogue-maker build`): read the view embedded in the page.
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import type { ReadView } from "./lib/bindings/ReadView";
  import type { Session } from "./lib/bindings/Session";
  import { Editor } from "./lib/editor.svelte";
  import FormatsBar from "./lib/FormatsBar.svelte";
  import GitBar from "./lib/GitBar.svelte";
  import { t } from "./lib/i18n.svelte";
  import Reader from "./lib/Reader.svelte";

  const LAST = "dialogue-maker:last-project";

  let view: ReadView | null = $state.raw(embedded());
  let path: string | null = $state(null);
  let error: string | null = $state(null);
  let loading = $state(false);
  const desktop = isTauri();
  // In the desktop app, every answer from the app is a session: the view and the edits.
  const editor = desktop ? new Editor((session) => (view = session.view)) : undefined;

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
      editor!.receive(await invoke<Session>("open_project", { path: folder }));
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
    const folder = await open({ directory: true, title: t("Open a game project") });
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
        editor!.receive(await invoke<Session>("reload"));
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
  <Reader {view} {editor}>
    {#snippet actions()}
      {#if desktop}
        <button class="btn" type="button" onclick={choose}>{t("Open another project…")}</button>
        {#if path}<span class="pmeta">{path} · {t("follows changes on disk")}</span>{/if}
        {#if view}<FormatsBar {view} />{/if}
        <GitBar />
        {#if error}<span class="error">{error}</span>{/if}
      {/if}
    {/snippet}
  </Reader>
{:else}
  <main class="welcome gut">
    <h1>dialogue-maker</h1>
    {#if desktop}
      <p>{t("Open a game project's folder: the app finds its dialogue files and shows the script. Nothing on disk changes until you save.")}</p>
      <button class="btn primary" type="button" onclick={choose} disabled={loading}>{loading ? t("Opening…") : t("Open a project…")}</button>
      {#if error}<p class="error">{error}</p>{/if}
    {:else}
      <p>{t("No script in this page.")}</p>
    {/if}
  </main>
{/if}
