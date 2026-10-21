<script lang="ts">
  // One line of dialogue: face, speaker, text, and what its tags say. In edit mode, its
  // tools, and the editor when it is open on this line.
  import type { Line } from "./bindings/Line";
  import type { Mark } from "./bindings/Mark";
  import type { Span } from "./bindings/Span";
  import { useEditor } from "./editor.svelte";
  import Face from "./Face.svelte";
  import Hl from "./Hl.svelte";
  import JumpView from "./JumpView.svelte";
  import LineEditor from "./LineEditor.svelte";
  import LineTools from "./LineTools.svelte";
  import Marks from "./Marks.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";
  import TagChips from "./TagChips.svelte";
  import TextView from "./TextView.svelte";

  let { line, marks, span }: { line: Line; marks: Mark[]; span?: Span } = $props();
  const reader = useReader();
  const editor = useEditor();
  const thought = $derived(reader.shown(line.tags, "thought").length > 0);
  const character = $derived(line.speaker ? reader.characters.get(line.speaker) : undefined);
  const editing = $derived(editor?.on && span !== undefined);
  const openOn = (mode: "edit" | "before" | "after") => editor && span && editor.is(span.file, span.start, mode);
</script>

{#if editor && span && openOn("edit")}
  <LineEditor target={editor.target!} speaker={line.speaker} words={line.words} tags={line.tags} />
{:else}
  {#if editor && span && openOn("before")}<LineEditor target={editor.target!} speaker={line.speaker} />{/if}
  <div
    class="ln"
    class:thought
    class:marked={marks.length > 0}
    class:concurrent={line.concurrent}
    class:editable={editing}
    role={editing ? "group" : undefined}
    ondblclick={() => {
      if (editing && editor && span) editor.target = { file: span.file, at: span.start, mode: "edit", choice: false };
    }}
  >
    <span class="avw"><Face image={reader.face(line.speaker, line.tags)} name={line.speaker} /></span>
    <div class="lb">
      <div class="who">
        {#if line.speaker}<span title={character?.name}><Hl text={line.speaker} /></span>{:else}<span class="narration">{t("narration")}</span>{/if}
        <TagChips tags={line.tags} />
        {#if line.concurrent}<span class="tid">{t("at the same time")}</span>{/if}
        <Marks {marks} />
        {#if line.id && reader.showIds}<span class="tid">{line.id}</span>{/if}
      </div>
      <div class="say"><TextView segments={line.text} /></div>
      {#if line.jump}<JumpView jump={line.jump} inline />{/if}
    </div>
    {#if editing && span}<LineTools {span} choice={false} />{/if}
  </div>
  {#if editor && span && openOn("after")}
    <LineEditor target={editor.target!} speaker={editor.target!.choice ? null : line.speaker} />
  {/if}
{/if}
