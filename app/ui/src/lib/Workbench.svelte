<script lang="ts">
  // The app around a script: the bar at the top (the views, undo and redo, import and
  // export, git and GitHub, settings, saving) and the view chosen under it.
  import { untrack } from "svelte";
  import type { ReadView as View } from "./bindings/ReadView";
  import DiffPanel from "./DiffPanel.svelte";
  import { type Editor, provideEditor } from "./editor.svelte";
  import ExportDialog from "./ExportDialog.svelte";
  import { plural, t } from "./i18n.svelte";
  import ImportDialog from "./ImportDialog.svelte";
  import ProjectPanel from "./ProjectPanel.svelte";
  import { Reader, type Tab, provideReader } from "./reader.svelte";
  import ReadView from "./ReadView.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import ShareButton from "./ShareButton.svelte";
  import StoryMap from "./StoryMap.svelte";
  import WriteView from "./write/WriteView.svelte";

  let { view, editor, path = null, openOther }: { view: View; editor?: Editor; path?: string | null; openOther?: () => void } = $props();
  // The editor exists in the desktop app only; the static page reads.
  provideEditor(untrack(() => editor));
  // Later views (a reload) go in through the effect below; the reader keeps its state.
  const reader = new Reader(
    untrack(() => view),
    untrack(() => editor !== undefined),
  );
  provideReader(reader);
  $effect(() => {
    reader.view = view;
  });

  // Each face once, as a class, instead of a data URI on every line. A constructed
  // stylesheet, because the app's content security policy refuses `<style>` elements.
  const faces = new CSSStyleSheet();
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, faces];
  $effect(() => {
    faces.replaceSync(reader.imageCss);
  });

  $effect(() => {
    if (reader.theme === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = reader.theme;
  });

  // Lines open for editing in the Write view only.
  $effect(() => {
    if (editor) editor.on = reader.tab === "write";
  });

  const tabs: { id: Tab; name: () => string; desktop: boolean }[] = [
    { id: "write", name: () => t("Write"), desktop: true },
    { id: "read", name: () => t("Read"), desktop: false },
    { id: "map", name: () => t("Map"), desktop: false },
    { id: "project", name: () => t("Project"), desktop: true },
  ];

  let exporting = $state(false);
  let importing = $state(false);
  let settings = $state(false);
  let reviewing = $state(false);
  const changed = $derived(editor?.state.changed.length ?? 0);
  const conflicts = $derived(editor?.state.conflicts.length ?? 0);

  // Undo, redo and save from the keyboard; text fields keep their own undo.
  function keydown(e: KeyboardEvent) {
    if (!editor || !(e.ctrlKey || e.metaKey)) return;
    const typing = e.target instanceof HTMLElement && e.target.closest("input, textarea, select");
    const key = e.key.toLowerCase();
    if (key === "s") {
      e.preventDefault();
      if (editor.state.changed.length > 0) editor.save();
    } else if (!typing && key === "z") {
      e.preventDefault();
      if (e.shiftKey) editor.redo();
      else editor.undo();
    } else if (!typing && key === "y") {
      e.preventDefault();
      editor.redo();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="app">
  <header class="topbar">
    <div class="brand">
      <div class="eyebrow">{editor ? t("Script editor") : t("The whole script")}</div>
      <div class="title" title={path ?? undefined}>{reader.view.title}</div>
    </div>
    <div class="tabs" role="tablist" aria-label={t("Views")}>
      {#each tabs.filter((tab) => editor || !tab.desktop) as tab (tab.id)}
        <button type="button" role="tab" aria-selected={reader.tab === tab.id} onclick={() => (reader.tab = tab.id)}>{tab.name()}</button>
      {/each}
    </div>
    <div class="top-tools">
      {#if editor}
        <button class="btn" type="button" onclick={editor.undo} disabled={!editor.state.can_undo || editor.busy} title={t("Undo (Ctrl+Z)")}>{t("Undo")}</button>
        <button class="btn" type="button" onclick={editor.redo} disabled={!editor.state.can_redo || editor.busy} title={t("Redo (Ctrl+Shift+Z)")}>{t("Redo")}</button>
        <span class="sep" aria-hidden="true"></span>
        <button class="btn" type="button" onclick={() => (importing = true)}>{t("Import")}</button>
        <button class="btn" type="button" onclick={() => (exporting = true)}>{t("Export")}</button>
        <ShareButton />
      {/if}
      <button class="btn" type="button" onclick={() => (settings = true)}>{t("Settings")}</button>
      {#if editor}
        <span class="save-state" class:dirty={changed > 0} class:conflict={conflicts > 0}>
          {#if conflicts > 0}
            <button type="button" class="link" onclick={() => (reviewing = true)}>{t("Changed on disk")}</button>
          {:else if changed > 0}
            <button type="button" class="link" onclick={() => (reviewing = true)} title={t("See the changes before saving")}>{plural(changed, "{n} file not saved", "{n} files not saved")}</button>
          {:else}
            <span>{t("All changes saved")}</span>
          {/if}
          {#if changed > 0}<button class="btn primary" type="button" onclick={editor.save} disabled={editor.busy} title={t("Save (Ctrl+S)")}>{t("Save")}</button>{/if}
        </span>
      {/if}
    </div>
  </header>

  <main class="stage" class:page={reader.tab === "read" || reader.tab === "project"}>
    {#if editor && reader.tab === "write"}
      <WriteView />
    {:else if reader.tab === "map"}
      <StoryMap />
    {:else if editor && reader.tab === "project"}
      <ProjectPanel />
    {:else}
      <ReadView />
    {/if}
  </main>

  {#if editor?.error && !editor.target}
    <div class="toast error" role="alert">
      <span>{editor.error}</span>
      <button type="button" class="link" onclick={() => editor && (editor.error = null)}>{t("Dismiss")}</button>
    </div>
  {/if}
</div>

{#if exporting}<ExportDialog close={() => (exporting = false)} />{/if}
{#if importing}<ImportDialog close={() => (importing = false)} />{/if}
{#if settings}<SettingsDialog close={() => (settings = false)} {path} {openOther} />{/if}
{#if reviewing}<DiffPanel close={() => (reviewing = false)} />{/if}
