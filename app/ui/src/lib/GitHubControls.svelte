<script lang="ts">
  // GitHub, in the app: sign in (the browser asks you to authorize dialogue-maker), then
  // push the branch and open its pull request. Nothing reaches GitHub until you ask.
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Account } from "./bindings/Account";
  import type { PullRequest } from "./bindings/PullRequest";
  import type { Repo } from "./bindings/Repo";
  import type { SignIn } from "./bindings/SignIn";

  let { head }: { head: string | null } = $props();

  let repo: Repo | null = $state(null);
  let account: Account | null = $state(null);
  let error: string | null = $state(null);

  async function load() {
    try {
      const state = await invoke<{ repo: Repo | null; account: Account | null }>("github_state");
      repo = state.repo;
      account = state.account;
    } catch (e) {
      error = String(e);
    }
  }
  // Again when the last commit changes: a commit, another branch, another project.
  $effect(() => {
    void head;
    load();
  });

  // Signing in.
  let signIn: SignIn | null = $state(null);
  let copied = $state(false);
  async function startSignIn() {
    error = null;
    try {
      signIn = await invoke<SignIn>("github_sign_in_start");
      copied = false;
      account = await invoke<Account>("github_sign_in_finish");
      signIn = null;
    } catch (e) {
      if (signIn) error = String(e);
      signIn = null;
    }
  }
  async function openGitHub() {
    if (!signIn) return;
    try {
      await navigator.clipboard.writeText(signIn.user_code);
      copied = true;
    } catch {
      copied = false;
    }
    await openUrl(signIn.verification_uri);
  }
  async function cancelSignIn() {
    signIn = null;
    await invoke("github_sign_in_cancel");
  }
  async function signOut() {
    await invoke("github_sign_out");
    account = null;
  }

  // The pull request.
  let asking = $state(false);
  let title = $state("");
  let body = $state("");
  let busy = $state(false);
  let result: PullRequest | null = $state(null);
  function openDialog() {
    title = head?.replace(/^[0-9a-f]{7} /, "") ?? "";
    body = "Script changes made with dialogue-maker.";
    result = null;
    error = null;
    asking = true;
  }
  async function pullRequest() {
    busy = true;
    error = null;
    try {
      result = await invoke<PullRequest>("github_pull_request", { title, body });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if repo}
  <span class="gh">
    <span class="k">GitHub</span>
    <span class="tid">{repo.owner}/{repo.name}</span>
    {#if account}
      <button class="btn" type="button" onclick={openDialog}>Pull request…</button>
      <span class="tid">as {account.login}</span>
      <button type="button" class="link" onclick={signOut}>sign out</button>
    {:else}
      <button class="btn" type="button" onclick={startSignIn}>Sign in to GitHub</button>
    {/if}
    {#if error && !asking && !signIn}<span class="error">{error}</span>{/if}
  </span>
{/if}

{#if signIn}
  <div class="overlay" role="presentation">
    <div class="diff-panel sign-in" role="dialog" aria-label="Sign in to GitHub" aria-modal="true">
      <h2>Sign in to GitHub</h2>
      <p>GitHub asks you to authorize dialogue-maker. Give it this code there:</p>
      <p class="user-code">{signIn.user_code}</p>
      <div class="le-actions">
        <button class="btn primary" type="button" onclick={openGitHub}>Copy the code and open GitHub</button>
        <button class="btn" type="button" onclick={cancelSignIn}>Cancel</button>
      </div>
      <p class="tid">
        {copied ? "Copied. " : ""}Waiting for you to authorize on {signIn.verification_uri} … dialogue-maker asks to push to your repositories and open pull
        requests. You can revoke it any time in GitHub's settings, under Applications.
      </p>
      {#if error}<p class="error">{error}</p>{/if}
    </div>
  </div>
{/if}

{#if asking}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && !busy && (asking = false)}>
    <div class="diff-panel" role="dialog" aria-label="Pull request" aria-modal="true">
      <h2>Pull request</h2>
      {#if result}
        <p>Pull request #{result.number} {result.created ? "opened" : "was already open; your commits are on it now"}.</p>
        <div class="le-actions">
          <button class="btn primary" type="button" onclick={() => result && openUrl(result.url)}>Open it on GitHub</button>
          <button class="btn" type="button" onclick={() => (asking = false)}>Close</button>
        </div>
      {:else}
        <p class="lede">Pushes this branch's commits to GitHub and opens a pull request for the team to review. Nothing is forced: if GitHub has commits you do not, you are told.</p>
        <form class="commit-form" onsubmit={(e) => { e.preventDefault(); pullRequest(); }}>
          <label class="le-field"><span class="k">Title</span><input bind:value={title} /></label>
          <label class="le-field"><span class="k">Description</span><textarea bind:value={body} rows="4"></textarea></label>
          {#if error}<p class="error">{error}</p>{/if}
          <div class="le-actions">
            <button class="btn primary" type="submit" disabled={busy}>{busy ? "Pushing…" : "Push and open pull request"}</button>
            <button class="btn" type="button" disabled={busy} onclick={() => (asking = false)}>Cancel</button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
