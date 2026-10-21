<script lang="ts">
  // Import: another tool's script (or a Dialogue Manager file from elsewhere) as a new file
  // in the project, after a look at it and at what did not convert.
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Imported } from "./bindings/Imported";
  import type { Session } from "./bindings/Session";
  import Dialog from "./Dialog.svelte";
  import { useEditor } from "./editor.svelte";
  import { plural, t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";

  let { close }: { close: () => void } = $props();
  const reader = useReader();
  const editor = useEditor()!;

  let source: string | null = $state(null);
  let imported: Imported | null = $state(null);
  let destination = $state("");
  let error: string | null = $state(null);
  let done: string | null = $state(null);

  /** The folder of the project's first dialogue file: where new ones likely go. */
  function dialogueFolder(): string {
    const first = reader.view.script.files[0]?.path ?? "";
    const at = first.lastIndexOf("/");
    return at > 0 ? first.slice(0, at + 1) : "";
  }

  async function choose() {
    error = null;
    done = null;
    const path = await open({
      title: t("Import a script"),
      filters: [{ name: t("Twine, Yarn Spinner, Ink or Dialogue Manager"), extensions: ["twee", "tw", "yarn", "ink", "dialogue"] }],
    });
    if (typeof path !== "string") return;
    try {
      imported = await invoke<Imported>("import_preview", { path });
      source = path;
      const stem = path.replace(/^.*[\\/]/, "").replace(/\.[^.]+$/, "");
      destination = `${dialogueFolder()}${stem}.dialogue`;
    } catch (e) {
      error = String(e);
      source = null;
      imported = null;
    }
  }

  async function add() {
    if (!source) return;
    error = null;
    try {
      const result = await invoke<{ session: Session; included: boolean }>("import_write", { source, path: destination });
      editor.receive(result.session);
      done = result.included
        ? t("Added {path}.", { path: destination })
        : t("Wrote {path}, but the project file's sources do not take it in: add its folder to [sources] dialogue.", { path: destination });
      source = null;
      imported = null;
    } catch (e) {
      error = String(e);
    }
  }
</script>

<Dialog title={t("Import")} {close} wide={imported !== null}>
  {#if imported && source}
    <p class="lede">{t("{source}, as a Dialogue Manager file. It is added as a new file; nothing in the project changes.", { source })}</p>
    {#if imported.warnings.length}
      <details open class="import-warnings">
        <summary>{plural(imported.warnings.length, "{n} thing did not convert, kept as a comment", "{n} things did not convert, kept as comments")}</summary>
        <ul>
          {#each imported.warnings as w, i (i)}<li><span class="tid">{t("line {n}", { n: w.line })}</span> {w.message}</li>{/each}
        </ul>
      </details>
    {/if}
    <pre class="file-preview">{imported.text}</pre>
    <label class="field"><span class="k">{t("Add to the project as")}</span><input bind:value={destination} spellcheck="false" /></label>
  {:else}
    <p class="lede">{t("Bring a script from another tool into the project, as a new Dialogue Manager file. You see it, and what did not convert, before anything is written.")}</p>
    <ul class="formats-list">
      <li><b>Twine</b> <span class="tid">.twee .tw</span> — {t("Twee 3, which Twine 2 exports")}</li>
      <li><b>Yarn Spinner</b> <span class="tid">.yarn</span></li>
      <li><b>Ink</b> <span class="tid">.ink</span></li>
      <li><b>Dialogue Manager</b> <span class="tid">.dialogue</span> — {t("a file from elsewhere, as it is")}</li>
    </ul>
    {#if done}<p class="ok">{done}</p>{/if}
  {/if}
  {#if error}<p class="error">{error}</p>{/if}

  {#snippet actions()}
    {#if imported && source}
      <button class="btn primary" type="button" onclick={add}>{t("Add to the project")}</button>
      <button class="btn" type="button" onclick={choose}>{t("Choose another file…")}</button>
    {:else}
      <button class="btn primary" type="button" onclick={choose}>{t("Choose a file…")}</button>
    {/if}
  {/snippet}
</Dialog>
