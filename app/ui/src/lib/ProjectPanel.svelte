<script lang="ts">
  // The project file's builders, one tab each: everything the dialogue files cannot say.
  import CharactersBuilder from "./CharactersBuilder.svelte";
  import { useEditor } from "./editor.svelte";
  import FlagsBuilder from "./FlagsBuilder.svelte";
  import { t } from "./i18n.svelte";
  import PlacesBuilder from "./PlacesBuilder.svelte";
  import TagsBuilder from "./TagsBuilder.svelte";
  import VariantsBuilder from "./VariantsBuilder.svelte";

  const editor = useEditor()!;
  type Part = "characters" | "tags" | "flags" | "places" | "variants";
  let part: Part = $state("characters");
  const parts: { id: Part; name: () => string; about: () => string }[] = [
    { id: "characters", name: () => t("Characters"), about: () => t("Who speaks: their names in the script, portraits, faces and voice moods.") },
    { id: "tags", name: () => t("Tags"), about: () => t("What the tags on lines mean, and how the app shows them.") },
    { id: "flags", name: () => t("Flags"), about: () => t("What each flag means, where it is set and read.") },
    { id: "places", name: () => t("Places"), about: () => t("Where scenes play: the order of the script, and the groups places go in.") },
    { id: "variants", name: () => t("Variants"), about: () => t("Ways the game can be played (a mode, a difficulty), to read one at a time.") },
  ];
  const current = $derived(parts.find((p) => p.id === part)!);
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
    <div class="project-head">
      <div class="tabs inline" role="tablist" aria-label={t("Builders")}>
        {#each parts as p (p.id)}
          <button type="button" role="tab" aria-selected={part === p.id} onclick={() => (part = p.id)}>{p.name()}</button>
        {/each}
      </div>
      <p class="hint">{current.about()} {t("Saved in {file}, keeping its comments and order.", { file: editor.state.project_file })}</p>
    </div>
    {#if part === "characters"}<CharactersBuilder />
    {:else if part === "tags"}<TagsBuilder />
    {:else if part === "flags"}<FlagsBuilder />
    {:else if part === "places"}<PlacesBuilder />
    {:else}<VariantsBuilder />{/if}
  {/if}
</div>
