// GitHub, in the app: who is signed in, the open project's repository on it, signing in
// (GitHub's device flow: the browser asks the writer to authorize easyscript). Shared
// by the welcome screen and the share panel.

import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Account } from "./bindings/Account";
import type { Repo } from "./bindings/Repo";
import type { SignIn } from "./bindings/SignIn";

export type GitHubState = { git: boolean; origin: string | null; repo: Repo | null; account: Account | null };

class GitHub {
  state: GitHubState = $state({ git: false, origin: null, repo: null, account: null });
  signIn: SignIn | null = $state(null);
  copied = $state(false);
  error: string | null = $state(null);

  async load() {
    try {
      this.state = await invoke<GitHubState>("github_state");
    } catch (e) {
      this.error = String(e);
    }
  }

  /** Sign in; resolves with the account, or null when cancelled or refused. */
  async startSignIn(): Promise<Account | null> {
    this.error = null;
    try {
      this.signIn = await invoke<SignIn>("github_sign_in_start");
      this.copied = false;
      const account = await invoke<Account>("github_sign_in_finish");
      this.state = { ...this.state, account };
      this.signIn = null;
      return account;
    } catch (e) {
      if (this.signIn) this.error = String(e);
      this.signIn = null;
      return null;
    }
  }

  async openGitHub() {
    if (!this.signIn) return;
    try {
      await navigator.clipboard.writeText(this.signIn.user_code);
      this.copied = true;
    } catch {
      this.copied = false;
    }
    await openUrl(this.signIn.verification_uri);
  }

  async cancelSignIn() {
    this.signIn = null;
    await invoke("github_sign_in_cancel");
  }

  async signOut() {
    await invoke("github_sign_out");
    this.state = { ...this.state, account: null };
  }
}

export const github = new GitHub();
