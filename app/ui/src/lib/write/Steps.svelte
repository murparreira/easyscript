<script lang="ts">
  // A scene's steps to write: lines as cards, the player's choices, conditions, effects and
  // jumps, nested ones inside their choice or condition. Each can be opened to change it,
  // moved, removed, or added to.
  import type { Node } from "../bindings/Node";
  import type { Span } from "../bindings/Span";
  import ConditionInput from "../ConditionInput.svelte";
  import { useEditor } from "../editor.svelte";
  import Expr from "../Expr.svelte";
  import FlagChip from "../FlagChip.svelte";
  import { t } from "../i18n.svelte";
  import LineEditor from "../LineEditor.svelte";
  import Marks from "../Marks.svelte";
  import { armSides, conditionSide, exprPieces } from "../model";
  import Notes from "../Notes.svelte";
  import { useReader } from "../reader.svelte";
  import JumpPill from "./JumpPill.svelte";
  import StepLine from "./StepLine.svelte";
  import Steps from "./Steps.svelte";
  import Tools from "./Tools.svelte";

  let { nodes }: { nodes: Node[] } = $props();
  const reader = useReader();
  const editor = useEditor()!;

  // The condition being edited, by where its line starts, and its new text.
  let conditionAt: number | null = $state(null);
  let conditionText = $state("");
  async function saveCondition(file: number, at: number) {
    const path = reader.view.script.files[file].path;
    if ((await editor.edit(path, [{ op: "set_condition", at, condition: conditionText.trim() || null }])) != null) conditionAt = null;
  }
  const opened = (span: Span, mode: "edit" | "before" | "after" | "under") => editor.target !== null && editor.is(span.file, span.start, mode);
  const add = (span: Span, choice: boolean) => (editor.target = { file: span.file, at: span.start, mode: "under", choice });
</script>

{#snippet condition(span: Span, text: string | null, keyword: string)}
  {#if conditionAt === span.start}
    <form class="cond-edit" onsubmit={(e) => { e.preventDefault(); saveCondition(span.file, span.start); }}>
      <ConditionInput bind:value={conditionText} label={keyword} />
      <button class="btn primary" type="submit" disabled={editor.busy}>{t("Apply")}</button>
      <button class="btn" type="button" onclick={() => (conditionAt = null)}>{t("Cancel")}</button>
    </form>
  {:else}
    <span class="kw">{keyword}</span>
    {#if text}<Expr expr={text} />{/if}
    <button type="button" class="icon" title={t("Edit the condition")} aria-label={t("Edit the condition")} onclick={() => { conditionAt = span.start; conditionText = text ?? ""; }}>✎</button>
  {/if}
{/snippet}

{#each nodes as node, i (i)}
  {#if opened(node.span, "before")}<LineEditor target={editor.target!} />{/if}
  <Notes notes={node.notes} kind="w-note" />
  {#if node.type === "line"}
    <StepLine line={node} marks={node.marks} span={node.span} />
  {:else if node.type === "menu"}
    <div class="w-menu">
      <div class="w-menu-head">{t("The player chooses")}</div>
      {#each node.choices as choice, j (j)}
        {@const side = conditionSide(choice.condition, reader.view.variants)}
        {#if !reader.hides(side)}
          <div class="w-choice">
            <Notes notes={choice.notes} kind="w-note" />
            {#if opened(choice.span, "before")}<LineEditor target={editor.target!} />{/if}
            {#if opened(choice.span, "edit")}
              <LineEditor target={editor.target!} speaker={choice.speaker} words={choice.words} tags={choice.tags} condition={choice.condition} jump={choice.jump} />
            {:else}
              <div class="w-choice-row" class:marked={choice.marks.length > 0}>
                <button type="button" class="w-choice-text" onclick={() => (editor.target = { file: choice.span.file, at: choice.span.start, mode: "edit", choice: true })}>
                  <span class="arrow" aria-hidden="true">▸</span>
                  {#if choice.speaker}<span class="who-inline">{choice.speaker}:</span>{/if}
                  {choice.words.replace(/\\n/g, " ").replace(/\\([[\]:#])/g, "$1")}
                </button>
                <div class="w-choice-meta">
                  {#if side}<span class="chip variant" class:yes={side.side}>{side.side ? side.variant.if_true : side.variant.if_false}</span>
                  {:else if choice.condition}<span class="when">{t("only if")}</span> <Expr expr={choice.condition} />{/if}
                  <Marks marks={choice.marks} />
                  {#if choice.jump}<JumpPill jump={choice.jump} />{/if}
                </div>
                <Tools span={choice.span} choice={true} nested={!choice.jump} />
              </div>
            {/if}
            {#if choice.body.length > 0 || opened(choice.span, "under")}
              <div class="w-nest">
                <Steps nodes={choice.body} />
                {#if opened(choice.span, "under")}<LineEditor target={editor.target!} />{/if}
              </div>
            {/if}
            {#if opened(choice.span, "after")}<LineEditor target={editor.target!} />{/if}
          </div>
        {/if}
      {/each}
      <div class="w-menu-foot">
        <button type="button" class="btn small" onclick={() => { const c = node.choices[node.choices.length - 1]; editor.target = { file: c.span.file, at: c.span.start, mode: "after", choice: true }; }}>{t("+ Choice")}</button>
      </div>
    </div>
  {:else if node.type === "branch"}
    {@const sides = armSides(node.arms, reader.view.variants)}
    <div class="w-cond">
      {#each node.arms as arm, j (j)}
        {#if !reader.hides(sides[j])}
          <div class="w-arm">
            <Notes notes={arm.notes} kind="w-note" />
            <div class="w-arm-head">
              {#if sides[j]}
                {@const side = sides[j]!}
                <span class="chip variant" class:yes={side.side}>{side.side ? side.variant.if_true : side.variant.if_false}</span>
              {:else if arm.keyword === "else"}
                <span class="kw">{t("Otherwise")}</span>
              {:else}
                {@render condition(arm.span, arm.condition, arm.keyword === "elif" ? t("Or if") : t("If"))}
              {/if}
              <Marks marks={arm.marks} />
              <span class="grow"></span>
              <button type="button" class="btn small" onclick={() => add(arm.span, false)}>{t("+ Line")}</button>
            </div>
            <div class="w-nest">
              {#if arm.body.length > 0}<Steps nodes={arm.body} />{:else}<p class="nothing">{t("nothing")}</p>{/if}
              {#if opened(arm.span, "under")}<LineEditor target={editor.target!} />{/if}
            </div>
          </div>
        {/if}
      {/each}
    </div>
    {#if opened(node.arms[node.arms.length - 1].span, "after")}<LineEditor target={editor.target!} />{/if}
  {:else if node.type === "match"}
    {@const flags = reader.view.flag_functions.match_subjects.includes(node.subject)}
    <div class="w-cond">
      <div class="w-arm-head match"><span class="kw">{t("Depends on")}</span> <Expr expr={node.subject} /></div>
      {#each node.arms as arm, j (j)}
        <div class="w-arm">
          <Notes notes={arm.notes} kind="w-note" />
          <div class="w-arm-head">
            {#if arm.keyword === "when"}
              {#if flags && arm.condition && conditionAt !== arm.span.start}
                <span class="kw">{t("When")}</span> <FlagChip flag={arm.condition.replace(/^&?["']|["']$/g, "")} />
                <button type="button" class="icon" title={t("Edit the condition")} aria-label={t("Edit the condition")} onclick={() => { conditionAt = arm.span.start; conditionText = arm.condition ?? ""; }}>✎</button>
              {:else}
                {@render condition(arm.span, arm.condition, t("When"))}
              {/if}
            {:else}
              <span class="kw">{t("Otherwise")}</span>
            {/if}
            <span class="grow"></span>
            <button type="button" class="btn small" onclick={() => add(arm.span, false)}>{t("+ Line")}</button>
          </div>
          <div class="w-nest">
            <Steps nodes={arm.body} />
            {#if opened(arm.span, "under")}<LineEditor target={editor.target!} />{/if}
          </div>
        </div>
      {/each}
    </div>
    {#if opened(node.arms[node.arms.length - 1].span, "after")}<LineEditor target={editor.target!} />{/if}
  {:else if node.type === "while"}
    <div class="w-cond">
      <div class="w-arm-head">{@render condition(node.span, node.condition, t("While"))}</div>
      <div class="w-nest"><Steps nodes={node.body} /></div>
    </div>
  {:else if node.type === "random"}
    <div class="w-cond">
      <div class="w-arm-head match"><span class="kw">{t("One of these, at random")}</span></div>
      {#each node.options as option, j (j)}
        <div class="w-arm">
          {#if option.weight || option.condition}
            <div class="w-arm-head">
              {#if option.weight}<span class="kw">{t("weight {weight}", { weight: option.weight })}</span>{/if}
              {#if option.condition}<span class="when">{t("if")}</span> <Expr expr={option.condition} />{/if}
            </div>
          {/if}
          <div class="w-nest"><Steps nodes={option.body} /></div>
        </div>
      {/each}
    </div>
    {#if opened(node.options[node.options.length - 1].span, "after")}<LineEditor target={editor.target!} />{/if}
  {:else if node.type === "effect"}
    {@const pieces = exprPieces(node.expr, reader.view.flag_functions)}
    <div class="w-row w-fx">
      <span class="kw">{pieces.some((p) => "flag" in p && p.access === "set") ? t("Sets") : t("Does")}</span>
      <Expr expr={node.expr} />
      <Marks marks={node.marks} />
      <Tools span={node.span} choice={false} edit={false} />
    </div>
  {:else if node.type === "jump"}
    <div class="w-row w-jump">
      <JumpPill jump={node} />
      <Tools span={node.span} choice={false} jump={node} />
    </div>
  {:else if node.type === "anchor"}
    <div class="w-row w-anchor"><span class="kw">{t("Anchor")}</span> <code>~ {node.id}</code></div>
  {/if}
  {#if node.type !== "line" && opened(node.span, "after")}<LineEditor target={editor.target!} />{/if}
{/each}
