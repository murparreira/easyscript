// The team, through the project's git `origin`: what came in that this copy does not
// have, what this copy has that origin does not, and what teammates are changing on their
// own branches, all told by scene. The app asks origin now and then (a fetch, which changes
// nothing of the writer's); bringing changes in and sending are the writer's to ask.

import { invoke } from "@tauri-apps/api/core";
import { getContext, setContext } from "svelte";
import type { BranchWork } from "./bindings/BranchWork";
import type { CommitInfo } from "./bindings/CommitInfo";
import type { PullRequest } from "./bindings/PullRequest";
import type { SceneRef } from "./bindings/SceneRef";
import type { Session } from "./bindings/Session";
import type { TeamState } from "./bindings/TeamState";
import type { Updated } from "./bindings/Updated";
import type { Editor } from "./editor.svelte";
import { plural, t } from "./i18n.svelte";
import { fileName } from "./model";

export type Sent = { branch: string; pull_request: PullRequest | null };

const AUTO = "dialogue-maker:team-auto";
const SEEN = "dialogue-maker:seen:";
const ACK = "dialogue-maker:seen-scenes:";
/** How often origin is asked, and at least how long apart when the window comes back. */
const EVERY = 3 * 60_000;
const AGAIN = 60_000;

/** A scene as a key: its file and label. */
export const sceneKey = (path: string, label: string | null) => `${path}#${label ?? ""}`;

/** How a scene is named in a list: its label, or its file's name. */
export const sceneName = (scene: SceneRef) => scene.label ?? fileName(scene.path);

/** How long ago a time (seconds since 1970) was, in words. */
export function ago(time: number, now = Date.now()): string {
  const minutes = Math.max(0, Math.round((now / 1000 - time) / 60));
  if (minutes < 1) return t("just now");
  if (minutes < 60) return plural(minutes, "{n} minute ago", "{n} minutes ago");
  const hours = Math.round(minutes / 60);
  if (hours < 24) return plural(hours, "{n} hour ago", "{n} hours ago");
  return plural(Math.round(hours / 24), "{n} day ago", "{n} days ago");
}

function byScene<T>(items: T[], scenes: (item: T) => SceneRef[]): Map<string, T[]> {
  const out = new Map<string, T[]>();
  for (const item of items) {
    for (const s of scenes(item)) {
      const key = sceneKey(s.path, s.label);
      out.set(key, [...(out.get(key) ?? []), item]);
    }
  }
  return out;
}

function stored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function store(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Not remembered; it still applies now.
  }
}

export class Team {
  state: TeamState | null = $state(null);
  /** When origin was last asked (milliseconds), and why it could not be reached. */
  checked: number | null = $state(null);
  checking = $state(false);
  unreachable: string | null = $state(null);
  busy = $state(false);
  /** Ask origin every few minutes. */
  auto = $state(stored(AUTO) !== "off");
  /** The last commit the writer saw. */
  seen: string | null = $state(null);
  /** Scenes the team changed on origin that are not here yet. */
  incoming: Map<string, CommitInfo[]>;
  /** Scenes others changed here since the writer last looked. */
  since: Map<string, CommitInfo[]>;
  /** Scenes teammates are changing on their own branches (the writer's other branches
   *  are theirs to know). */
  others: Map<string, BranchWork[]>;
  #project: string;
  #editor: Editor;
  /** Scenes seen one by one: the latest commit to each when it was seen. */
  #acks: Record<string, string> = $state({});

  constructor(project: string, editor: Editor) {
    this.#project = project;
    this.#editor = editor;
    this.seen = stored(SEEN + project);
    try {
      this.#acks = JSON.parse(stored(ACK + project) ?? "{}") as Record<string, string>;
    } catch {
      this.#acks = {};
    }
    this.incoming = $derived(byScene(this.state?.incoming ?? [], (c) => c.scenes));
    this.since = $derived(
      new Map(
        [...byScene((this.state?.since ?? []).filter((c) => !c.mine), (c) => c.scenes)].filter(([key, commits]) => this.#acks[key] !== commits[0].id),
      ),
    );
    this.others = $derived(byScene((this.state?.others ?? []).filter((b) => !b.last.mine), (b) => b.last.scenes));
  }

  #take(state: TeamState | null) {
    this.state = state;
    // The first time, everything here counts as seen.
    if (state?.head && !this.seen) this.#see(state.head);
  }

  #see(id: string) {
    this.seen = id;
    store(SEEN + this.#project, id);
    this.#acks = {};
    store(ACK + this.#project, "{}");
  }

  /** One scene's changes count as seen (until someone changes it again). */
  seeScene(key: string) {
    const latest = this.since.get(key)?.[0];
    if (!latest) return;
    this.#acks = { ...this.#acks, [key]: latest.id };
    store(ACK + this.#project, JSON.stringify(this.#acks));
  }

  async load() {
    try {
      this.#take(await invoke<TeamState | null>("team_state", { seen: this.seen }));
    } catch {
      this.state = null;
    }
  }

  /** Ask origin what is new. */
  async check() {
    if (this.checking || !this.state) return;
    this.checking = true;
    try {
      this.#take(await invoke<TeamState | null>("team_fetch", { seen: this.seen }));
      this.unreachable = null;
    } catch (e) {
      this.unreachable = String(e);
    } finally {
      this.checked = Date.now();
      this.checking = false;
    }
  }

  /** Bring in the team's changes; null when it failed (the editor says why). */
  async bringIn(): Promise<Updated | null> {
    this.busy = true;
    this.#editor.error = null;
    try {
      const result = await invoke<{ updated: Updated; session: Session }>("team_update");
      this.#editor.receive(result.session, true);
      await this.load();
      return result.updated;
    } catch (e) {
      this.#editor.error = String(e);
      return null;
    } finally {
      this.busy = false;
    }
  }

  /** Send this branch's commits to origin (and open its pull request, when it can). */
  async send(title: string, body: string): Promise<Sent | null> {
    this.busy = true;
    this.#editor.error = null;
    try {
      const sent = await invoke<Sent>("team_send", { title, body });
      await this.load();
      return sent;
    } catch (e) {
      this.#editor.error = String(e);
      return null;
    } finally {
      this.busy = false;
    }
  }

  /** What changed until now counts as seen. */
  markSeen() {
    if (this.state?.head) this.#see(this.state.head);
    this.load();
  }

  setAuto(on: boolean) {
    this.auto = on;
    store(AUTO, on ? "on" : "off");
  }

  /** Ask origin now, then every few minutes and when the window comes back; returns the
   *  way to stop. */
  start(): () => void {
    this.load().then(() => {
      if (this.auto) return this.check();
    });
    const timer = setInterval(() => this.auto && this.check(), EVERY);
    const back = () => {
      if (this.auto && document.visibilityState === "visible" && Date.now() - (this.checked ?? 0) > AGAIN) this.check();
    };
    document.addEventListener("visibilitychange", back);
    window.addEventListener("focus", back);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", back);
      window.removeEventListener("focus", back);
    };
  }
}

const KEY = Symbol("team");

export function provideTeam(team: Team | undefined) {
  setContext(KEY, team);
}

/** The team, in the desktop app with a project open; undefined in the static page. */
export function useTeam(): Team | undefined {
  return getContext<Team | undefined>(KEY);
}
