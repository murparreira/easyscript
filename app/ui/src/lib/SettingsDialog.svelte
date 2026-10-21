<script lang="ts">
  // The app's settings: its language and look, what the script shows, the project open.
  import Dialog from "./Dialog.svelte";
  import { LANGUAGES, locale, t } from "./i18n.svelte";
  import { type Theme, useReader } from "./reader.svelte";

  let { close, path = null, openOther }: { close: () => void; path?: string | null; openOther?: () => void } = $props();
  const reader = useReader();
  const themes: { id: Theme; name: () => string }[] = [
    { id: "system", name: () => t("Like the system") },
    { id: "light", name: () => t("Light") },
    { id: "dark", name: () => t("Dark") },
  ];
</script>

<Dialog title={t("Settings")} {close}>
  <div class="settings">
    <label class="field">
      <span class="k">{t("Language")}</span>
      <select value={locale.language} onchange={(e) => locale.set(e.currentTarget.value as typeof locale.language)}>
        {#each LANGUAGES as l (l.id)}<option value={l.id}>{l.name}</option>{/each}
      </select>
    </label>
    <div class="field">
      <span class="k">{t("Look")}</span>
      <div class="seg" role="group" aria-label={t("Look")}>
        {#each themes as theme (theme.id)}
          <button type="button" aria-pressed={reader.theme === theme.id} onclick={() => (reader.theme = theme.id)}>{theme.name()}</button>
        {/each}
      </div>
    </div>
    <div class="field">
      <span class="k">{t("The script shows")}</span>
      <label class="tog"><input type="checkbox" bind:checked={reader.showIds} /> {t("Ids, tags and file names")}</label>
      <label class="tog"><input type="checkbox" bind:checked={reader.showMarks} /> {t("Lines with authorship marks, highlighted")}</label>
    </div>
    {#if openOther}
      <div class="field">
        <span class="k">{t("Project")}</span>
        {#if path}<code class="path">{path}</code>{/if}
        <p class="hint">{t("The app reads the dialogue files where they are, follows changes on disk, and writes only the files you change, when you save.")}</p>
        <div><button class="btn" type="button" onclick={() => { close(); openOther(); }}>{t("Open another project…")}</button></div>
      </div>
    {/if}
  </div>
</Dialog>
