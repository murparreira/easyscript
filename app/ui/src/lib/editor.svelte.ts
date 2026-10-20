// Editing, in the desktop app: edits go to the app's working copies of the files, which
// it writes only on save. The read view shows the working copies.

import { invoke } from "@tauri-apps/api/core";
import { getContext, setContext } from "svelte";
import type { Edit } from "./bindings/Edit";
import type { EditState } from "./bindings/EditState";
import type { FileDiff } from "./bindings/FileDiff";
import type { Session } from "./bindings/Session";

/** The line an editor is open on: editing it, or adding a line before or after it. */
export type Target = { file: number; at: number; mode: "edit" | "before" | "after"; choice: boolean };

export class Editor {
  state: EditState = $state({ changed: [], conflicts: [], can_undo: false, can_redo: false });
  /** Edit mode: lines can be opened and changed. */
  on = $state(false);
  target: Target | null = $state(null);
  busy = $state(false);
  error: string | null = $state(null);
  #show: (session: Session) => void;

  constructor(show: (session: Session) => void) {
    this.#show = show;
  }

  /** A new session from the app: show it and take its edit state. */
  receive(session: Session) {
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

const KEY = Symbol("editor");

export function provideEditor(editor: Editor | undefined) {
  setContext(KEY, editor);
}

/** The editor, in the desktop app; undefined in the static page. */
export function useEditor(): Editor | undefined {
  return getContext<Editor | undefined>(KEY);
}
