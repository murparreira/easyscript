<script lang="ts">
  // The app shell. In the desktop app: open a project folder (or clone one from GitHub),
  // and follow its files as they change. As a static page (`dialogue-maker build`): read
  // the view embedded in the page.
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { projects } from "./lib/app.svelte";
  import type { ReadView } from "./lib/bindings/ReadView";
  import type { Session } from "./lib/bindings/Session";
  import CloneDialog from "./lib/CloneDialog.svelte";
  import { Editor } from "./lib/editor.svelte";
  import { t } from "./lib/i18n.svelte";
  import Workbench from "./lib/Workbench.svelte";

  const LAST = "dialogue-maker:last-project";
  const RECENT = "dialogue-maker:recent-projects";

  let view: ReadView | null = $state.raw(embedded());
  let path: string | null = $state(null);
  let error: string | null = $state(null);
  let loading = $state(false);
  let cloning = $state(false);
  let recent: string[] = $state(readRecent());
  const desktop = isTauri();
  // In the desktop app, every answer from the app is a session: the view and the edits.
  const editor = desktop ? new Editor((session) => (view = session.view)) : undefined;

  function embedded(): ReadView | null {
    const data = document.getElementById("dm-view")?.textContent?.trim();
    return data ? (JSON.parse(data) as ReadView) : null;
  }

  function readRecent(): string[] {
    try {
      return JSON.parse(localStorage.getItem(RECENT) ?? "[]") as string[];
    } catch {
      return [];
    }
  }

  function remember(value: string | null) {
    try {
      if (value) {
        localStorage.setItem(LAST, value);
        recent = [value, ...recent.filter((r) => r !== value)].slice(0, 6);
        localStorage.setItem(RECENT, JSON.stringify(recent));
      } else localStorage.removeItem(LAST);
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
  projects.open = openProject;

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
        editor!.receive(await invoke<Session>("reload"), true);
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
  {#key path}
    <Workbench {view} {editor} {path} openOther={desktop ? choose : undefined} />
  {/key}
{:else}
  <main class="welcome">
    <div class="welcome-card">
      <div class="eyebrow">{t("Script editor")}</div>
      <h1>dialogue-maker</h1>
      {#if desktop}
        <p class="lede">{t("Write a game's dialogue where it lives. Open the game's folder: the app finds its dialogue files and shows the script. Nothing on disk changes until you save.")}</p>
        <div class="welcome-actions">
          <button class="btn primary big" type="button" onclick={choose} disabled={loading}>{loading ? t("Opening…") : t("Open a folder…")}</button>
          <button class="btn big" type="button" onclick={() => (cloning = true)} disabled={loading}>{t("Open from GitHub…")}</button>
        </div>
        {#if error}<p class="error">{error}</p>{/if}
        {#if recent.length > 0}
          <div class="recent">
            <div class="k">{t("Recent")}</div>
            {#each recent as folder (folder)}
              <button type="button" class="recent-item" onclick={() => openProject(folder)} disabled={loading}>
                <span class="name">{folder.split(/[\\/]/).pop()}</span>
                <span class="tid">{folder}</span>
              </button>
            {/each}
          </div>
        {/if}
      {:else}
        <p>{t("No script in this page.")}</p>
      {/if}
    </div>
  </main>
  {#if cloning}<CloneDialog close={() => (cloning = false)} opened={openProject} />{/if}
{/if}
