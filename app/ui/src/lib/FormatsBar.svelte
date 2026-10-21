<script lang="ts">
  // Exports (the script for other tools, translators, voice actors, screenwriting apps) and
  // imports (another tool's script as a new Dialogue Manager file in the project).
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import type { ExportFormat } from "./bindings/ExportFormat";
  import type { ExportOptions } from "./bindings/ExportOptions";
  import type { Imported } from "./bindings/Imported";
  import type { ReadView } from "./bindings/ReadView";
  import type { Session } from "./bindings/Session";
  import { useEditor } from "./editor.svelte";
  import { locale as app, plural, t } from "./i18n.svelte";

  let { view }: { view: ReadView } = $props();
  const editor = useEditor()!;

  // Names and file names in the writer's language, so a function each.
  const formats: { id: ExportFormat; name: () => string; about: () => string; ext: string; suffix: () => string }[] = [
    { id: "json", name: () => t("Script as JSON"), about: () => t("The whole script for other tools, in a documented format."), ext: "json", suffix: () => t("script") },
    {
      id: "translation",
      name: () => t("Translation table"),
      about: () => t("Dialogue Manager's CSV, which Godot imports. A table chosen that exists keeps its translations."),
      ext: "csv",
      suffix: () => t("translations"),
    },
    { id: "voice", name: () => t("Voice script"), about: () => t("Each character's lines by mood, with the line before for context (Markdown)."), ext: "md", suffix: () => t("voice script") },
    { id: "voice_csv", name: () => t("Recording sheet"), about: () => t("Every spoken line, one row each, to sort and fill in (CSV)."), ext: "csv", suffix: () => t("recording sheet") },
    { id: "fountain", name: () => t("Screenplay"), about: () => t("Fountain, which screenwriting apps open and print."), ext: "fountain", suffix: () => t("screenplay") },
  ];

  let exporting = $state(false);
  let format: ExportFormat = $state("json");
  let locale = $state("en");
  let translateTo = $state("");
  let character = $state("");
  let busy = $state(false);
  let error: string | null = $state(null);
  let note: string | null = $state(null);

  function openExport() {
    error = null;
    note = null;
    exporting = true;
  }

  async function runExport() {
    const chosen = formats.find((f) => f.id === format)!;
    const path = await save({
      title: chosen.name(),
      defaultPath: `${view.title} - ${chosen.suffix()}.${chosen.ext}`,
      filters: [{ name: chosen.name(), extensions: [chosen.ext] }],
    });
    if (!path) return;
    const options: ExportOptions = {
      locale: locale.trim() || null,
      extra_locales: translateTo
        .split(/[\s,]+/)
        .map((l) => l.trim())
        .filter(Boolean),
      character: character || null,
      language: app.language,
    };
    busy = true;
    error = null;
    try {
      await invoke("export_script", { format, options, path });
      note = t("Wrote {path}.", { path });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // Importing.
  let source: string | null = $state(null);
  let imported: Imported | null = $state(null);
  let destination = $state("");
  let importError: string | null = $state(null);
  let importNote: string | null = $state(null);

  /** The folder of the project's first dialogue file: where new ones likely go. */
  function dialogueFolder(): string {
    const first = view.script.files[0]?.path ?? "";
    const at = first.lastIndexOf("/");
    return at > 0 ? first.slice(0, at + 1) : "";
  }

  async function chooseImport() {
    importError = null;
    importNote = null;
    const path = await open({
      title: t("Import a script"),
      filters: [{ name: t("Twine, Yarn Spinner or Ink"), extensions: ["twee", "tw", "yarn", "ink"] }],
    });
    if (typeof path !== "string") return;
    try {
      imported = await invoke<Imported>("import_preview", { path });
      source = path;
      const stem = path.replace(/^.*[\\/]/, "").replace(/\.[^.]+$/, "");
      destination = `${dialogueFolder()}${stem}.dialogue`;
    } catch (e) {
      importError = String(e);
      source = null;
    }
  }

  async function addImport() {
    if (!source) return;
    importError = null;
    try {
      const done = await invoke<{ session: Session; included: boolean }>("import_write", { source, path: destination });
      editor.receive(done.session);
      importNote = done.included
        ? t("Added {path}.", { path: destination })
        : t("Wrote {path}, but the project file's sources do not take it in: add its folder to [sources] dialogue.", { path: destination });
      source = null;
      imported = null;
    } catch (e) {
      importError = String(e);
    }
  }

  function closeImport() {
    source = null;
    imported = null;
  }
</script>

<span class="formats">
  <button class="btn" type="button" onclick={openExport}>{t("Export…")}</button>
  <button class="btn" type="button" onclick={chooseImport}>{t("Import…")}</button>
  {#if importNote}<span class="tid">{importNote}</span>{/if}
  {#if importError && !imported}<span class="error">{importError}</span>{/if}
</span>

{#if exporting}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && !busy && (exporting = false)}>
    <div class="diff-panel export" role="dialog" aria-label={t("Export")} aria-modal="true">
      <h2>{t("Export")}</h2>
      <p class="lede">{t("The script as it is in the app, edits not saved yet included.")}</p>
      <div class="format-list" role="radiogroup" aria-label={t("Format")}>
        {#each formats as f (f.id)}
          <label class="format-choice" class:on={format === f.id}>
            <input type="radio" name="format" value={f.id} bind:group={format} />
            <span><b>{f.name()}</b> <span class="tid">.{f.ext}</span><br /><span class="about">{f.about()}</span></span>
          </label>
        {/each}
      </div>
      {#if format === "translation"}
        <div class="le-row">
          <label class="le-field"><span class="k">{t("Written in")}</span><input bind:value={locale} placeholder="en" /></label>
          <label class="le-field"><span class="k">{t("Translated to")}</span><input bind:value={translateTo} placeholder="pt_BR, es" /></label>
        </div>
      {:else if format === "voice"}
        <label class="le-field">
          <span class="k">{t("Character")}</span>
          <select bind:value={character}>
            <option value="">{t("Everyone")}</option>
            {#each view.characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </label>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}
      {#if note}<p class="tid">{note}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="button" disabled={busy} onclick={runExport}>{busy ? t("Exporting…") : t("Export…")}</button>
        <button class="btn" type="button" disabled={busy} onclick={() => (exporting = false)}>{t("Close")}</button>
      </div>
    </div>
  </div>
{/if}

{#if imported && source}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && closeImport()}>
    <div class="diff-panel import" role="dialog" aria-label={t("Import")} aria-modal="true">
      <h2>{t("Import")}</h2>
      <p class="lede">{t("{source}, as a Dialogue Manager file. It is added as a new file; nothing in the project changes.", { source })}</p>
      {#if imported.warnings.length}
        <details open>
          <summary>{plural(imported.warnings.length, "{n} thing did not convert, kept as a comment", "{n} things did not convert, kept as comments")}</summary>
          <ul class="import-warnings">
            {#each imported.warnings as w, i (i)}<li><span class="tid">{t("line {n}", { n: w.line })}</span> {w.message}</li>{/each}
          </ul>
        </details>
      {/if}
      <pre class="import-preview">{imported.text}</pre>
      <label class="le-field"><span class="k">{t("Add to the project as")}</span><input bind:value={destination} /></label>
      {#if importError}<p class="error">{importError}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="button" onclick={addImport}>{t("Add to the project")}</button>
        <button class="btn" type="button" onclick={closeImport}>{t("Cancel")}</button>
      </div>
    </div>
  </div>
{/if}
