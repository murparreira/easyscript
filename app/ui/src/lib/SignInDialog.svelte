<script lang="ts">
  // The code to give GitHub while signing in, until the writer authorizes or cancels.
  import Dialog from "./Dialog.svelte";
  import { github } from "./github.svelte";
  import { t } from "./i18n.svelte";
</script>

{#if github.signIn}
  <Dialog title={t("Sign in to GitHub")} close={() => github.cancelSignIn()}>
    <p>{t("GitHub asks you to authorize dialogue-maker. Give it this code there:")}</p>
    <p class="user-code">{github.signIn.user_code}</p>
    <p class="hint">
      {github.copied ? `${t("Copied.")} ` : ""}{t("Waiting for you to authorize on {site}… dialogue-maker asks to push to your repositories and open pull requests. You can revoke it any time in GitHub's settings, under Applications.", { site: github.signIn.verification_uri })}
    </p>
    {#if github.error}<p class="error">{github.error}</p>{/if}
    {#snippet actions()}
      <button class="btn primary" type="button" onclick={() => github.openGitHub()}>{t("Copy the code and open GitHub")}</button>
    {/snippet}
  </Dialog>
{/if}
