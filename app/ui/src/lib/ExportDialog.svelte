<script lang="ts">
  // Export: the dialogue files for the game (saved in place, or as a ZIP that keeps the
  // game's folders), and the script for others: translators, voice actors, screenwriting
  // apps, other tools.
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import type { ExportFormat } from "./bindings/ExportFormat";
  import type { ExportOptions } from "./bindings/ExportOptions";
  import Dialog from "./Dialog.svelte";
  import { useEditor } from "./editor.svelte";
  import { locale as app, plural, t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  let { close }: { close: () => void } = $props();
  const reader = useReader();
  const editor = useEditor()!;

  type Tab = "game" | "translation" | "voice" | "fountain" | "json";
  let tab = $state<Tab>("game");
  const tabs: { id: Tab; name: () => string }[] = [
    { id: "game", name: () => t("For the game") },
    { id: "translation", name: () => t("Translation") },
    { id: "voice", name: () => t("Voice") },
    { id: "fountain", name: () => t("Screenplay") },
    { id: "json", name: () => t("Data (JSON)") },
  ];
  let busy = $state(false);
  let error: string | null = $state(null);
  let note: string | null = $state(null);

  // The dialogue files.
  const files = $derived(
    reader.view.script.files.map((f, index) => {
      const labels = f.sections.flatMap((s) => s.labels);
      const places = [...new Set(labels.map((l) => l.place).filter(Boolean))].map((id) => reader.view.places.find((p) => p.id === id)?.name ?? id);
      return { index, path: f.path, scenes: labels.length, places: places.join(", ") };
    }),
  );
  let chosen: string[] = $state([]);
  const all = $derived(chosen.length === 0 || chosen.length === files.length);
  let previewing: string | null = $state(null);
  let preview = $state("");
  async function show(path: string) {
    previewing = path;
    try {
      preview = await invoke<string>("dialogue_text", { path });
    } catch (e) {
      preview = "";
      error = String(e);
    }
  }
  const status = (path: string) =>
    editor.state.conflicts.includes(path) ? t("changed on disk") : editor.state.changed.includes(path) ? t("edited, not saved") : t("saved");

  async function saveFiles() {
    busy = true;
    error = null;
    if (await editor.save()) note = t("Saved to the game's folder.");
    busy = false;
  }
  async function zip() {
    const path = await save({
      title: t("Save the dialogue files as a ZIP"),
      defaultPath: `${reader.view.title} - ${t("dialogue")}.zip`,
      filters: [{ name: "ZIP", extensions: ["zip"] }],
    });
    if (!path) return;
    busy = true;
    error = null;
    try {
      await invoke("export_dialogue_zip", { paths: all ? [] : chosen, path });
      note = t("Wrote {path}.", { path });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function copy() {
    try {
      await navigator.clipboard.writeText(preview);
      note = t("Copied {path}.", { path: previewing ?? "" });
    } catch (e) {
      error = String(e);
    }
  }

  // The script for others.
  let locale = $state("en");
  let translateTo = $state("");
  let character = $state("");
  let sheet = $state(false);
  const format: ExportFormat = $derived(tab === "voice" ? (sheet ? "voice_csv" : "voice") : tab === "game" ? "json" : tab);
  const kinds: Record<ExportFormat, { ext: string; suffix: () => string }> = {
    json: { ext: "json", suffix: () => t("script") },
    translation: { ext: "csv", suffix: () => t("translations") },
    voice: { ext: "md", suffix: () => t("voice script") },
    voice_csv: { ext: "csv", suffix: () => t("recording sheet") },
    fountain: { ext: "fountain", suffix: () => t("screenplay") },
  };
  async function exportScript() {
    const kind = kinds[format];
    const path = await save({
      title: tabs.find((x) => x.id === tab)?.name(),
      defaultPath: `${reader.view.title} - ${kind.suffix()}.${kind.ext}`,
      filters: [{ name: kind.suffix(), extensions: [kind.ext] }],
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
  function choose(next: Tab) {
    tab = next;
    error = null;
    note = null;
  }
</script>

<Dialog title={t("Export")} {close} {busy} wide>
  <div class="tabs inline" role="tablist" aria-label={t("Export")}>
    {#each tabs as x (x.id)}
      <button type="button" role="tab" aria-selected={tab === x.id} onclick={() => choose(x.id)}>{x.name()}</button>
    {/each}
  </div>

  {#if tab === "game"}
    <p class="lede">
      {t("The game reads these files. Saving writes your edits into them, in the game's folder. To hand them over another way, save them as a ZIP: it keeps the game's folders, so it unzips over the project.")}
    </p>
    <div class="table-wrap">
      <table class="files-table">
        <thead>
          <tr>
            <th><input type="checkbox" aria-label={t("All files")} checked={chosen.length === files.length} onchange={(e) => (chosen = e.currentTarget.checked ? files.map((f) => f.path) : [])} /></th>
            <th>{t("File")}</th>
            <th>{t("Place")}</th>
            <th class="num">{t("Scenes")}</th>
            <th>{t("Status")}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each files as file (file.path)}
            <tr class:on={previewing === file.path}>
              <td><input type="checkbox" aria-label={file.path} checked={chosen.includes(file.path)} onchange={(e) => (chosen = e.currentTarget.checked ? [...chosen, file.path] : chosen.filter((p) => p !== file.path))} /></td>
              <td><code>{file.path.split("/").pop()}</code></td>
              <td>{file.places}</td>
              <td class="num">{file.scenes}</td>
              <td class:dirty={editor.state.changed.includes(file.path)}>{status(file.path)}</td>
              <td><button class="btn small" type="button" onclick={() => show(file.path)}>{t("Preview")}</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if previewing}
      <div class="preview-head"><span class="k">{previewing}</span></div>
      <pre class="file-preview">{preview}</pre>
    {/if}
  {:else if tab === "translation"}
    <p class="lede">{t("Dialogue Manager's CSV, which Godot imports. Choosing a table that exists keeps its translations, and the rows of lines no longer in the script.")}</p>
    <div class="le-row">
      <label class="field"><span class="k">{t("Written in")}</span><input bind:value={locale} placeholder="en" /></label>
      <label class="field"><span class="k">{t("Translated to")}</span><input bind:value={translateTo} placeholder="pt_BR, es" /></label>
    </div>
  {:else if tab === "voice"}
    <p class="lede">{t("For voice actors: each character's lines by mood, with the line before for context.")}</p>
    <div class="seg" role="group" aria-label={t("Voice")}>
      <button type="button" aria-pressed={!sheet} onclick={() => (sheet = false)}>{t("Voice script (Markdown)")}</button>
      <button type="button" aria-pressed={sheet} onclick={() => (sheet = true)}>{t("Recording sheet (CSV)")}</button>
    </div>
    {#if !sheet}
      <label class="field">
        <span class="k">{t("Character")}</span>
        <select bind:value={character}>
          <option value="">{t("Everyone")}</option>
          {#each reader.view.characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </label>
    {/if}
  {:else if tab === "fountain"}
    <p class="lede">{t("Fountain, which screenwriting apps (Highland, Fade In, Slugline, Beat) open and print. Each scene a scene; choices and jumps as directions.")}</p>
  {:else}
    <p class="lede">{t("The whole script for other tools, in a documented format (docs/formats).")}</p>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if note}<p class="ok">{note}</p>{/if}

  {#snippet actions()}
    {#if tab === "game"}
      <button class="btn primary" type="button" onclick={saveFiles} disabled={busy || editor.state.changed.length === 0}>
        {editor.state.changed.length > 0 ? plural(editor.state.changed.length, "Save {n} file to the game", "Save {n} files to the game") : t("All saved")}
      </button>
      <button class="btn" type="button" onclick={zip} disabled={busy}>{all ? t("Save all as ZIP…") : plural(chosen.length, "Save {n} file as ZIP…", "Save {n} files as ZIP…")}</button>
      <button class="btn" type="button" onclick={copy} disabled={!previewing}>{t("Copy previewed file")}</button>
    {:else}
      <button class="btn primary" type="button" disabled={busy} onclick={exportScript}>{busy ? t("Exporting…") : t("Export…")}</button>
    {/if}
  {/snippet}
</Dialog>
