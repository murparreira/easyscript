<script lang="ts">
  // Edit a line or a choice, or write a new one: speaker, words, face, voice, the tags
  // shown as thought or shake, a choice's condition, where it goes next; with a preview of
  // the balloon.
  import { onMount, untrack } from "svelte";
  import Balloon from "./Balloon.svelte";
  import type { Edit } from "./bindings/Edit";
  import type { Jump } from "./bindings/Jump";
  import type { Tag } from "./bindings/Tag";
  import ConditionInput from "./ConditionInput.svelte";
  import { type Target, useEditor } from "./editor.svelte";
  import FacePicker from "./FacePicker.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";
  import JumpPicker from "./write/JumpPicker.svelte";

  let {
    target,
    speaker: speakerBefore = null,
    words: wordsBefore = "",
    tags: tagsBefore = [],
    condition: conditionBefore = null,
    jump: jumpBefore = null,
  }: { target: Target; speaker?: string | null; words?: string; tags?: Tag[]; condition?: string | null; jump?: Jump | null } = $props();
  const reader = useReader();
  const editor = useEditor()!;

  /** A jump as the picker writes it: a title, `END`, `END!`; empty for none. */
  function written(jump: Jump | null): string | null {
    if (!jump) return "";
    if (jump.returns || jump.target.kind === "expression") return null;
    return jump.target.kind === "label" ? jump.target.value : jump.target.kind === "end" ? "END" : "END!";
  }

  // The editor works on the line as it was when it opened. A tag's value is null when the
  // tag has none (`[#thought]`).
  const initial = untrack(() => ({
    speaker: speakerBefore,
    words: wordsBefore,
    tags: Object.fromEntries(tagsBefore.map((t) => [t.name, t.value])) as Record<string, string | null>,
    condition: conditionBefore,
    // Null: a jump the picker cannot show (with return, to an expression); left as it is.
    jump: written(jumpBefore),
  }));
  const before = initial.tags;
  // Drawn again over the same line (it moved in the file): what the writer had typed.
  const draft = untrack(() => editor.draft);
  let speaker = $state(draft?.speaker ?? initial.speaker ?? "");
  let words = $state(draft?.words ?? initial.words);
  let tags: Record<string, string | null> = $state({ ...(draft?.tags ?? before) });
  let condition = $state(draft?.condition ?? initial.condition ?? "");
  let goTo = $state(draft?.goTo ?? initial.jump ?? "");

  const specs = $derived(reader.view.tags);
  const face = $derived(specs.find((s) => s.show === "face" && s.kind === "asset"));
  const voice = $derived(specs.find((s) => s.show === "voice" && s.kind === "enum"));
  const switches = $derived(specs.filter((s) => s.kind === "flag"));
  const shownByControls = $derived(new Set([face?.name, voice?.name, ...switches.map((s) => s.name)].filter(Boolean)));
  const otherTags = $derived(Object.keys(tags).filter((name) => !shownByControls.has(name)));

  let faceValue = $state(untrack(() => (draft ? draft.face : face && face.name in before ? before[face.name] : null)));
  $effect(() => {
    editor.draft = { speaker, words, tags: { ...tags }, condition, goTo, face: faceValue };
  });
  $effect(() => {
    if (!face) return;
    if (faceValue) tags[face.name] = faceValue;
    else delete tags[face.name];
  });
  let picking = $state(false);

  const has = (name: string) => name in tags;
  const showing = (show: string) => switches.some((s) => s.show === show && has(s.name));
  const faceClass = $derived(
    face?.path && faceValue ? (reader.imageClasses.get(face.path.replace("{value}", faceValue)) ?? null) : reader.face(speaker || null, []),
  );
  const path = $derived(reader.view.script.files[target.file].path);
  const isNew = $derived(target.mode !== "edit");
  const title = $derived(
    isNew
      ? target.choice
        ? target.mode === "under"
          ? t("New choice inside")
          : target.mode === "after"
            ? t("New choice below")
            : t("New choice above")
        : target.mode === "under"
          ? t("New line inside")
          : target.mode === "after"
            ? t("New line below")
            : t("New line above")
      : target.choice
        ? t("Edit choice")
        : t("Edit line"),
  );

  function toggle(name: string, on: boolean) {
    if (on) tags[name] = null;
    else delete tags[name];
  }

  function setVoice(value: string) {
    if (!voice) return;
    if (value) tags[voice.name] = value;
    else delete tags[voice.name];
  }

  async function save() {
    const at = target.at;
    let edits: Edit[] = [];
    if (isNew) {
      edits = [
        {
          op: "insert_line",
          at,
          after: target.mode === "after",
          under: target.mode === "under",
          line: {
            choice: target.choice,
            speaker: speaker.trim() || null,
            text: words,
            tags: Object.entries(tags).map(([name, value]) => ({ name, value })),
            condition: target.choice ? condition.trim() || null : null,
            jump: goTo || null,
          },
        },
      ];
    } else {
      if ((speaker.trim() || null) !== initial.speaker) edits.push({ op: "set_speaker", at, speaker: speaker.trim() || null });
      if (words !== initial.words) edits.push({ op: "set_text", at, text: words });
      for (const name of new Set([...Object.keys(before), ...Object.keys(tags)])) {
        if (!(name in tags)) edits.push({ op: "remove_tag", at, name });
        else if (!(name in before) || before[name] !== tags[name]) edits.push({ op: "set_tag", at, name, value: tags[name] });
      }
      if (target.choice && (condition.trim() || null) !== initial.condition) {
        edits.push({ op: "set_condition", at, condition: condition.trim() || null });
      }
      if (initial.jump !== null && goTo !== initial.jump) edits.push({ op: "set_jump", at, target: goTo || null });
    }
    if (edits.length === 0) {
      editor.target = null;
      return;
    }
    if ((await editor.edit(path, edits)) !== null) editor.target = null;
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      editor.target = null;
    } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      save();
    }
  }

  let wordsBox: HTMLTextAreaElement | undefined = $state();
  onMount(() => {
    wordsBox?.focus();
    wordsBox?.closest("form")?.scrollIntoView({ block: "nearest" });
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form class="line-editor" class:choice={target.choice} onsubmit={(e) => { e.preventDefault(); save(); }} onkeydown={keydown}>
  <div class="le-head">
    <span class="k">{title}</span>
    {#if reader.showIds}<span class="tid">{path}</span>{/if}
  </div>
  <div class="le-grid">
    <div class="le-fields">
      <div class="le-row">
        <label class="field inline grow"><span class="k">{t("Speaker")}</span>
          <input type="text" bind:value={speaker} list="dm-speakers" placeholder={t("narration")} spellcheck="false" />
        </label>
        <datalist id="dm-speakers">{#each reader.speakers as name (name)}<option value={name}></option>{/each}</datalist>
        {#if voice}
          <label class="field inline"><span class="k">{t("Voice")}</span>
            <select value={tags[voice.name] ?? ""} onchange={(e) => setVoice(e.currentTarget.value)}>
              <option value="">{t("plain")}</option>
              {#each voice.values as v (v)}<option value={v}>{v}</option>{/each}
            </select>
          </label>
        {/if}
      </div>
      <label class="field"><span class="k">{target.choice ? t("What the player picks") : t("Words")}</span>
        <textarea bind:this={wordsBox} bind:value={words} rows={Math.min(10, Math.max(3, Math.ceil(words.length / 46) + 1))} spellcheck="true"></textarea>
      </label>
      <div class="le-row">
        {#each switches as s (s.name)}
          <label class="tog" title={s.description ?? ""}>
            <input type="checkbox" checked={has(s.name)} onchange={(e) => toggle(s.name, e.currentTarget.checked)} />
            {s.show === "thought" ? t("thought") : s.show === "shake" ? t("shake") : (s.show ?? s.name)}
          </label>
        {/each}
        {#if face && face.available.length > 0}
          <button type="button" class="chip-btn" aria-expanded={picking} onclick={() => (picking = !picking)}>
            {t("Face")}: {faceValue ?? t("none")} {picking ? "▴" : "▾"}
          </button>
        {/if}
        {#each otherTags as name (name)}
          <span class="chip">{name}{tags[name] ? `=${tags[name]}` : ""}
            <button type="button" class="link" aria-label={t("Remove {name}", { name })} onclick={() => delete tags[name]}>✕</button></span>
        {/each}
      </div>
      {#if target.choice}<ConditionInput bind:value={condition} label={t("Shown if")} />{/if}
      {#if initial.jump !== null || isNew}<JumpPicker bind:value={goTo} />{/if}
    </div>
    <div class="le-side">
      <Balloon face={faceClass} speaker={speaker || null} {words} thought={showing("thought")} shake={showing("shake")} />
    </div>
  </div>
  {#if face && picking}<FacePicker tag={face.name} speaker={speaker || null} bind:value={faceValue} />{/if}
  {#if editor.error}<p class="error">{editor.error}</p>{/if}
  <div class="le-actions">
    <button class="btn primary" type="submit" disabled={editor.busy}>{isNew ? t("Add") : t("Apply")}</button>
    <button class="btn" type="button" onclick={() => (editor.target = null)}>{t("Cancel")}</button>
    <span class="hint">{t("Ctrl+Enter applies · Esc cancels · nothing is written until you save")}</span>
  </div>
</form>
