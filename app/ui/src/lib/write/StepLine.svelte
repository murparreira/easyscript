<script lang="ts">
  // A line to write: the speaker's face and name, how it is said (thought, voice, face),
  // its words; a click opens it.
  import type { Line } from "../bindings/Line";
  import type { Mark } from "../bindings/Mark";
  import type { Span } from "../bindings/Span";
  import { useEditor } from "../editor.svelte";
  import Face from "../Face.svelte";
  import { t } from "../i18n.svelte";
  import LineEditor from "../LineEditor.svelte";
  import Marks from "../Marks.svelte";
  import { useReader } from "../reader.svelte";
  import TextView from "../TextView.svelte";
  import JumpPill from "./JumpPill.svelte";
  import Tools from "./Tools.svelte";

  let { line, marks, span }: { line: Line; marks: Mark[]; span: Span } = $props();
  const reader = useReader();
  const editor = useEditor()!;
  const thought = $derived(reader.shown(line.tags, "thought").length > 0);
  const shake = $derived(reader.shown(line.tags, "shake").length > 0);
  const voice = $derived(reader.shown(line.tags, "voice")[0]?.value ?? null);
  const face = $derived(line.tags.find((tag) => reader.tagSpec(tag.name)?.show === "face")?.value ?? null);
  const other = $derived(line.tags.filter((tag) => !reader.tagSpec(tag.name)?.show));
  const character = $derived(line.speaker ? reader.characters.get(line.speaker) : undefined);
  const editing = $derived(editor.target !== null && editor.is(span.file, span.start, "edit"));

  function open() {
    editor.target = { file: span.file, at: span.start, mode: "edit", choice: false };
  }
</script>

{#if editing}
  <LineEditor target={editor.target!} speaker={line.speaker} words={line.words} tags={line.tags} jump={line.jump} />
{:else}
  <div class="w-line" class:thought class:marked={marks.length > 0} class:concurrent={line.concurrent}>
    <Face image={reader.face(line.speaker, line.tags)} name={line.speaker} />
    <div class="w-line-body">
      <div class="w-line-head">
        {#if line.speaker}<span class="who" title={character?.name}>{line.speaker}</span>{:else}<span class="who narration">{t("Narration")}</span>{/if}
        {#if thought}<span class="chip">{t("thought")}</span>{/if}
        {#if shake}<span class="chip">{t("shake")}</span>{/if}
        {#if voice}<span class="chip" title={t("Voice: {mood}", { mood: voice })}>{t("voice: {mood}", { mood: voice })}</span>{/if}
        {#if face && reader.showIds}<span class="chip quiet">{face}</span>{/if}
        {#if reader.showIds}{#each other as tag, i (i)}<span class="chip quiet">{tag.name}{tag.value ? `=${tag.value}` : ""}</span>{/each}{/if}
        {#if line.concurrent}<span class="chip quiet">{t("at the same time")}</span>{/if}
        <Marks {marks} />
        {#if line.id && reader.showIds}<span class="tid">{line.id}</span>{/if}
      </div>
      <button type="button" class="w-say" onclick={open} title={t("Click to edit")}><TextView segments={line.text} /></button>
      {#if line.jump}<div><JumpPill jump={line.jump} /></div>{/if}
    </div>
    <Tools {span} choice={false} edit={false} />
  </div>
  {#if editor.target && editor.is(span.file, span.start, "after")}
    <LineEditor target={editor.target} speaker={editor.target.choice ? null : line.speaker} />
  {/if}
{/if}
