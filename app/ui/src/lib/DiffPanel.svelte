<script lang="ts">
  // The unsaved edits as they will change the files; save them, or drop them. A file that
  // changed on disk in the same lines as the edits shows both, to keep one side.
  import { invoke } from "@tauri-apps/api/core";
  import { ask } from "@tauri-apps/plugin-dialog";
  import type { FileDiff } from "./bindings/FileDiff";
  import type { Session } from "./bindings/Session";
  import Dialog from "./Dialog.svelte";
  import { useEditor } from "./editor.svelte";
  import { plural, t } from "./i18n.svelte";

  let { close }: { close: () => void } = $props();
  const editor = useEditor()!;
  let diffs: FileDiff[] | null = $state(null);
  $effect(() => {
    // Again whenever the edits change.
    void editor.state;
    editor.diffs().then((d) => (diffs = d));
  });

  // What changed on disk, for each file in conflict.
  let theirs: Record<string, FileDiff | null> = $state({});
  $effect(() => {
    for (const path of editor.state.conflicts) {
      if (!(path in theirs)) invoke<FileDiff | null>("disk_diff", { path }).then((d) => (theirs = { ...theirs, [path]: d }));
    }
  });
  async function settle(path: string, keep: boolean) {
    if (!keep) {
      const sure = await ask(t("Drop your edits to {file} and take the version on disk?", { file: path }), { title: t("Take the version on disk"), kind: "warning" });
      if (!sure) return;
    }
    editor.busy = true;
    try {
      editor.receive(await invoke<Session>("resolve_conflict", { path, keepEdits: keep }), true);
      const { [path]: _, ...rest } = theirs;
      theirs = rest;
    } catch (e) {
      editor.error = String(e);
    } finally {
      editor.busy = false;
    }
  }

  async function save() {
    if (await editor.save()) close();
  }

  async function discard() {
    const n = editor.state.changed.length;
    const sure = await ask(plural(n, "Drop the unsaved edits to {n} file? This cannot be undone.", "Drop the unsaved edits to {n} files? This cannot be undone."), {
      title: t("Discard edits"),
      kind: "warning",
    });
    if (sure && (await editor.discard())) close();
  }
</script>

<Dialog title={t("Unsaved changes")} {close} busy={editor.busy} wide>
  {#each editor.state.conflicts as path (path)}
    <section class="diff-file conflict">
      <h3><code>{path}</code> <span class="error">{t("changed on disk")}</span></h3>
      <p>{t("Someone changed this file on disk (a teammate's change brought in, the Godot editor) in the same lines as your unsaved edits. Keep one side: yours below, or theirs here.")}</p>
      {#if theirs[path]}
        <div class="k">{t("Their change")}</div>
        {#each theirs[path]?.hunks ?? [] as hunk, i (i)}
          <pre class="hunk">{#each hunk as line, j (j)}<span class="dl {line.kind}">{line.kind === "added" ? "+" : line.kind === "removed" ? "-" : " "} {line.text}
</span>{/each}</pre>
        {/each}
      {/if}
      <div class="le-row">
        <button class="btn" type="button" onclick={() => settle(path, true)} disabled={editor.busy}>{t("Keep my edits")}</button>
        <button class="btn" type="button" onclick={() => settle(path, false)} disabled={editor.busy}>{t("Take theirs")}</button>
      </div>
    </section>
  {/each}
  {#if diffs === null}
    <p class="nothing">{t("Reading…")}</p>
  {:else if diffs.length === 0}
    <p class="nothing">{t("Nothing to save.")}</p>
  {:else}
    {#each diffs as file (file.path)}
      <section class="diff-file">
        <h3><code>{file.path}</code>{#if editor.state.conflicts.includes(file.path)} <span class="k">{t("Your edits")}</span>{/if}</h3>
        {#each file.hunks as hunk, i (i)}
          <pre class="hunk">{#each hunk as line, j (j)}<span class="dl {line.kind}">{line.kind === "added" ? "+" : line.kind === "removed" ? "-" : " "} {line.text}
</span>{/each}</pre>
        {/each}
      </section>
    {/each}
  {/if}
  {#if editor.error}<p class="error">{editor.error}</p>{/if}
  <p class="hint">{t("Only these files are written; a file changed on disk since it was read is not.")}</p>

  {#snippet actions()}
    <button class="btn primary" type="button" onclick={save} disabled={editor.busy || !diffs?.length}>{plural(diffs?.length ?? 0, "Save {n} file", "Save {n} files")}</button>
    <button class="btn danger" type="button" onclick={discard} disabled={editor.busy || editor.state.changed.length === 0}>{t("Discard…")}</button>
  {/snippet}
</Dialog>
