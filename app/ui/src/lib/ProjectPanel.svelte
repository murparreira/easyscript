<script lang="ts">
  // The project file's builders: everything the dialogue files cannot say.
  import CharactersBuilder from "./CharactersBuilder.svelte";
  import { useEditor } from "./editor.svelte";
  import FlagsBuilder from "./FlagsBuilder.svelte";
  import PlacesBuilder from "./PlacesBuilder.svelte";
  import TagsBuilder from "./TagsBuilder.svelte";
  import VariantsBuilder from "./VariantsBuilder.svelte";

  const editor = useEditor()!;
</script>

<div class="project-panel">
  {#if !editor.state.has_project_file}
    <section class="sec">
      <h2>No project file yet</h2>
      <p class="lede">
        A project file (<code>{editor.state.project_file}</code>) says what the dialogue cannot: who the characters are, what the tags mean,
        the places, the flags. Draft one from what the dialogue already says, check it here, then save it.
      </p>
      <button class="btn primary" type="button" onclick={editor.draftProject} disabled={editor.busy}>Draft a project file</button>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
    </section>
  {:else}
    <p class="lede">Editing <code>{editor.state.project_file}</code>. Changes keep its comments and order, and are written on save.</p>
    <nav class="builder-nav" aria-label="Builders">
      <a href="#b-characters">Characters</a><a href="#b-tags">Tags</a><a href="#b-flags">Flags</a><a href="#b-places">Places</a><a href="#b-variants">Variants</a>
    </nav>
    <CharactersBuilder />
    <TagsBuilder />
    <FlagsBuilder />
    <PlacesBuilder />
    <VariantsBuilder />
  {/if}
</div>
