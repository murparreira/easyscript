<script lang="ts">
  // Open a game from GitHub: sign in, pick one of your repositories, pick where it goes;
  // the app clones it there and opens it, with `origin` pointing at it, so commits and pull
  // requests go back to it.
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { RepoInfo } from "./bindings/RepoInfo";
  import Dialog from "./Dialog.svelte";
  import { github } from "./github.svelte";
  import { t } from "./i18n.svelte";
  import RepoPicker from "./RepoPicker.svelte";
  import SignInDialog from "./SignInDialog.svelte";

  let { close, opened }: { close: () => void; opened: (path: string) => void } = $props();
  const PARENT = "easyscript:clone-parent";

  let repo = $state<RepoInfo | null>(null);
  let parent: string | null = $state(remembered());
  let busy = $state(false);
  let error: string | null = $state(null);
  let progress = $state<{ received: number; total: number } | null>(null);

  function remembered(): string | null {
    try {
      return localStorage.getItem(PARENT);
    } catch {
      return null;
    }
  }

  $effect(() => {
    github.load();
  });

  async function chooseParent() {
    const folder = await open({ directory: true, title: t("Where the game's folder goes") });
    if (typeof folder !== "string") return;
    parent = folder;
    try {
      localStorage.setItem(PARENT, folder);
    } catch {
      // The app just won't remember it.
    }
  }

  async function clone() {
    if (!repo || !parent) return;
    busy = true;
    error = null;
    progress = null;
    const stop = await listen<{ received: number; total: number }>("clone-progress", (e) => (progress = e.payload));
    try {
      const path = await invoke<string>("github_clone", { owner: repo.owner, name: repo.name, parent });
      opened(path);
      close();
    } catch (e) {
      error = String(e);
    } finally {
      stop();
      busy = false;
    }
  }
  const percent = $derived(progress && progress.total > 0 ? Math.round((100 * progress.received) / progress.total) : 0);
</script>

<Dialog title={t("Open from GitHub")} {close} {busy} wide>
  {#if !github.state.account}
    <p class="lede">{t("Sign in, then pick the game's repository: the app clones it into a folder you choose and opens it. Your commits and pull requests go back to that repository.")}</p>
    {#if github.error}<p class="error">{github.error}</p>{/if}
  {:else}
    <p class="lede">{t("Signed in as {login}. Pick the game's repository and where to put it.", { login: github.state.account.login })}</p>
    <RepoPicker bind:chosen={repo} />
    <div class="field">
      <span class="k">{t("Into")}</span>
      <div class="le-row">
        <button class="btn" type="button" onclick={chooseParent} disabled={busy}>{t("Choose a folder…")}</button>
        {#if parent}<code class="path">{parent}{repo ? `/${repo.name}` : ""}</code>{/if}
      </div>
    </div>
    {#if busy}
      <div class="progress" role="progressbar" aria-valuenow={percent} aria-valuemin="0" aria-valuemax="100"><span style:width="{percent}%"></span></div>
      <p class="hint">{progress ? t("Downloading: {done} of {total}", { done: progress.received, total: progress.total }) : t("Connecting to GitHub…")}</p>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  {/if}

  {#snippet actions()}
    {#if !github.state.account}
      <button class="btn primary" type="button" onclick={() => github.startSignIn()}>{t("Sign in to GitHub")}</button>
    {:else if busy}
      <button class="btn" type="button" onclick={() => invoke("github_clone_stop")}>{t("Stop")}</button>
    {:else}
      <button class="btn primary" type="button" onclick={clone} disabled={!repo || !parent}>{t("Clone and open")}</button>
    {/if}
  {/snippet}
</Dialog>
<SignInDialog />
