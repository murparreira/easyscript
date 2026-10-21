// Editing, in the desktop app: edits go to the app's working copies of the files, which
// it writes only on save. The read view shows the working copies.

import { invoke } from "@tauri-apps/api/core";
import { getContext, setContext } from "svelte";
import type { ConfigEdit } from "./bindings/ConfigEdit";
import type { Edit } from "./bindings/Edit";
import type { EditState } from "./bindings/EditState";
import type { FileDiff } from "./bindings/FileDiff";
import type { Label } from "./bindings/Label";
import type { Node } from "./bindings/Node";
import type { ReadView } from "./bindings/ReadView";
import type { Session } from "./bindings/Session";

/** The line an editor is open on: editing it, or adding a line before or after it, or
 *  under it (in a choice or a condition). */
export type Target = { file: number; at: number; mode: "edit" | "before" | "after" | "under"; choice: boolean };

/** A line editor's fields as the writer left them. */
export type Draft = { speaker: string; words: string; tags: Record<string, string | null>; condition: string; goTo: string; face: string | null };

export class Editor {
  state: EditState = $state({ changed: [], conflicts: [], can_undo: false, can_redo: false, project_file: "", has_project_file: false });
  /** Edit mode: lines can be opened and changed. */
  on = $state(false);
  #target: Target | null = $state(null);
  /** What the open line editor holds, so it survives the editor being drawn again (its
   *  line moved when the file changed on disk). */
  draft: Draft | null = null;
  busy = $state(false);
  error: string | null = $state(null);
  #show: (session: Session) => void;
  #view: ReadView | null = null;

  constructor(show: (session: Session) => void) {
    this.#show = show;
  }

  /** A new session from the app: show it and take its edit state. One that comes from
   *  outside (a file changed on disk, the team's changes brought in) may have moved the
   *  line open for editing: it is found again, or closed. */
  /** The line open for editing; opening another starts a new draft. */
  get target(): Target | null {
    return this.#target;
  }
  set target(target: Target | null) {
    this.#target = target;
    this.draft = null;
  }

  receive(session: Session, outside = false) {
    if (outside && this.#target && this.#view) {
      this.#target = follow(this.#target, this.#view, session.view);
      if (!this.#target) this.draft = null;
    }
    this.#view = session.view;
    this.#show(session);
    this.state = session.state;
  }

  async #run<T>(call: () => Promise<T>): Promise<T | null> {
    this.busy = true;
    this.error = null;
    try {
      return await call();
    } catch (e) {
      this.error = String(e);
      return null;
    } finally {
      this.busy = false;
    }
  }

  async #session(command: string): Promise<boolean> {
    const session = await this.#run(() => invoke<Session>(command));
    if (session) this.receive(session);
    return session !== null;
  }

  /** Make edits to a file as one step; the line the last one was about starts at the result. */
  async edit(path: string, edits: Edit[]): Promise<number | null> {
    const result = await this.#run(() => invoke<{ session: Session; at: number }>("edit", { path, edits }));
    if (!result) return null;
    this.receive(result.session);
    return result.at;
  }

  /** Edit the project file as one step. */
  async editProject(edits: ConfigEdit[]): Promise<boolean> {
    const session = await this.#run(() => invoke<Session>("edit_project_file", { edits }));
    if (session) this.receive(session);
    return session !== null;
  }

  draftProject = () => this.#session("draft_project_file");
  undo = () => this.#session("undo");
  redo = () => this.#session("redo");
  save = () => this.#session("save");
  discard = () => this.#session("discard");

  async diffs(): Promise<FileDiff[]> {
    return (await this.#run(() => invoke<FileDiff[]>("diffs"))) ?? [];
  }

  is(file: number, at: number, mode: Target["mode"]): boolean {
    const t = this.target;
    return t !== null && t.file === file && t.at === at && t.mode === mode;
  }
}

/** What a target can be on: a label's title, a step, a choice, an arm, an option; each
 *  with what it says, to find it again. */
function anchors(label: Label): { start: number; sig: string }[] {
  const out = [{ start: label.span.start, sig: "label" }];
  const sig = (x: object) =>
    JSON.stringify(x, (k, v) => (["span", "body", "choices", "arms", "options", "notes", "marks"].includes(k) ? undefined : v));
  const visit = (nodes: Node[]) => {
    for (const node of nodes) {
      out.push({ start: node.span.start, sig: sig(node) });
      if (node.type === "menu") for (const c of node.choices) (out.push({ start: c.span.start, sig: sig(c) }), visit(c.body));
      if (node.type === "branch" || node.type === "match") for (const a of node.arms) (out.push({ start: a.span.start, sig: sig(a) }), visit(a.body));
      if (node.type === "random") for (const o of node.options) (out.push({ start: o.span.start, sig: sig(o) }), visit(o.body));
      if (node.type === "while") visit(node.body);
    }
  };
  visit(label.body);
  return out;
}

const labelsOf = (view: ReadView, file: number) => view.script.files[file]?.sections.flatMap((s) => s.labels) ?? [];

/** A target in a view, in a newer one: the same thing (the nth that says the same) in the
 *  same label of the same file; none when it is gone. */
function follow(target: Target, before: ReadView, after: ReadView): Target | null {
  const path = before.script.files[target.file]?.path;
  const file = after.script.files.findIndex((f) => f.path === path);
  if (file < 0) return null;
  for (const label of labelsOf(before, target.file)) {
    const old = anchors(label);
    const i = old.findIndex((a) => a.start === target.at);
    if (i < 0) continue;
    const nth = old.slice(0, i).filter((a) => a.sig === old[i].sig).length;
    const now = labelsOf(after, file).find((l) => l.id === label.id);
    const found = now ? anchors(now).filter((a) => a.sig === old[i].sig)[nth] : undefined;
    return found ? { ...target, file, at: found.start } : null;
  }
  return null;
}

const KEY = Symbol("editor");

export function provideEditor(editor: Editor | undefined) {
  setContext(KEY, editor);
}

/** The editor, in the desktop app; undefined in the static page. */
export function useEditor(): Editor | undefined {
  return getContext<Editor | undefined>(KEY);
}
