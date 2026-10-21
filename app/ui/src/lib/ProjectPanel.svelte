<script lang="ts">
  // The project file's builders: everything the dialogue files cannot say.
  import CharactersBuilder from "./CharactersBuilder.svelte";
  import { useEditor } from "./editor.svelte";
  import FlagsBuilder from "./FlagsBuilder.svelte";
  import { t } from "./i18n.svelte";
  import PlacesBuilder from "./PlacesBuilder.svelte";
  import TagsBuilder from "./TagsBuilder.svelte";
  import VariantsBuilder from "./VariantsBuilder.svelte";

  const editor = useEditor()!;
</script>

<div class="project-panel">
  {#if !editor.state.has_project_file}
    <section class="sec">
      <h2>{t("No project file yet")}</h2>
      <p class="lede">
        {t("A project file ({file}) says what the dialogue cannot: who the characters are, what the tags mean, the places, the flags. Draft one from what the dialogue already says, check it here, then save it.", { file: editor.state.project_file })}
      </p>
      <button class="btn primary" type="button" onclick={editor.draftProject} disabled={editor.busy}>{t("Draft a project file")}</button>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
    </section>
  {:else}
    <p class="lede">{t("Editing {file}. Changes keep its comments and order, and are written on save.", { file: editor.state.project_file })}</p>
    <nav class="builder-nav" aria-label={t("Builders")}>
      <a href="#b-characters">{t("Characters")}</a><a href="#b-tags">{t("Tags")}</a><a href="#b-flags">{t("Flags")}</a><a href="#b-places">{t("Places")}</a><a href="#b-variants">{t("Variants")}</a>
    </nav>
    <CharactersBuilder />
    <TagsBuilder />
    <FlagsBuilder />
    <PlacesBuilder />
    <VariantsBuilder />
  {/if}
</div>
