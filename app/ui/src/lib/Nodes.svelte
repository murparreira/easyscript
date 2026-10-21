<script lang="ts">
  // A list of script steps, nested ones drawn inside their menus and branches.
  import type { Node } from "./bindings/Node";
  import Expr from "./Expr.svelte";
  import FlagChip from "./FlagChip.svelte";
  import Hl from "./Hl.svelte";
  import JumpView from "./JumpView.svelte";
  import LineView from "./LineView.svelte";
  import Marks from "./Marks.svelte";
  import { armSides, conditionSide } from "./model";
  import Nodes from "./Nodes.svelte";
  import Notes from "./Notes.svelte";
  import ConditionInput from "./ConditionInput.svelte";
  import { useEditor } from "./editor.svelte";
  import LineEditor from "./LineEditor.svelte";
  import LineTools from "./LineTools.svelte";
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";
  import TagChips from "./TagChips.svelte";
  import TextView from "./TextView.svelte";

  let { nodes }: { nodes: Node[] } = $props();
  const reader = useReader();
  const editor = useEditor();

  // The condition being edited, by where its line starts, and its new text.
  let conditionAt: number | null = $state(null);
  let conditionText = $state("");
  async function saveCondition(file: number, at: number) {
    const path = reader.view.script.files[file].path;
    if ((await editor?.edit(path, [{ op: "set_condition", at, condition: conditionText.trim() || null }])) != null) {
      conditionAt = null;
    }
  }
</script>

{#snippet conditionEditor(file: number, at: number, condition: string | null)}
  {#if editor?.on}
    {#if conditionAt === at}
      <form class="cond-edit" onsubmit={(e) => { e.preventDefault(); saveCondition(file, at); }}>
        <ConditionInput bind:value={conditionText} />
        <button class="btn primary" type="submit" disabled={editor.busy}>{t("Apply")}</button>
        <button class="btn" type="button" onclick={() => (conditionAt = null)}>{t("Cancel")}</button>
      </form>
    {:else}
      <button type="button" class="link" title={t("Edit the condition")} onclick={() => { conditionAt = at; conditionText = condition ?? ""; }}>✎</button>
    {/if}
  {/if}
{/snippet}

{#each nodes as node, i (i)}
  <Notes notes={node.notes} />
  {#if node.type === "line"}
    <LineView line={node} marks={node.marks} span={node.span} />
  {:else if node.type === "menu"}
    <div class="menu" class:thought={node.choices.every((c) => reader.shown(c.tags, "thought").length > 0)}>
      <div class="menu-h">{t("Choices")}</div>
      <ol>
        {#each node.choices as choice, j (j)}
          {@const side = conditionSide(choice.condition, reader.view.variants)}
          {#if !reader.hides(side)}
            <li>
              <Notes notes={choice.notes} />
              {#if editor?.is(choice.span.file, choice.span.start, "edit")}
                <LineEditor target={editor.target!} speaker={choice.speaker} words={choice.words} tags={choice.tags} condition={choice.condition} />
              {:else}
                {#if editor?.is(choice.span.file, choice.span.start, "before")}<LineEditor target={editor.target!} />{/if}
                <div class="opt-row" class:editable={editor?.on}>
                  <div>
                    <div class="opt" class:marked={choice.marks.length > 0}>
                      {#if choice.speaker}<span class="who-inline"><Hl text={choice.speaker} />:</span>{/if}
                      <TextView segments={choice.text} />
                    </div>
                    <div class="opt-meta">
                      {#if side}<span class="chip variant" class:yes={side.side}>{side.side ? side.variant.if_true : side.variant.if_false}</span>
                      {:else if choice.condition}<span class="when">{t("shown if")}</span> <Expr expr={choice.condition} />{/if}
                      <TagChips tags={choice.tags} />
                      <Marks marks={choice.marks} />
                      {#if choice.jump}<JumpView jump={choice.jump} inline />{/if}
                    </div>
                  </div>
                  {#if editor?.on}<LineTools span={choice.span} choice={true} />{/if}
                </div>
              {/if}
              {#if choice.body.length > 0}<div class="branch"><Nodes nodes={choice.body} /></div>{/if}
              {#if editor?.is(choice.span.file, choice.span.start, "after")}<LineEditor target={editor.target!} />{/if}
            </li>
          {/if}
        {/each}
      </ol>
    </div>
  {:else if node.type === "branch"}
    {@const sides = armSides(node.arms, reader.view.variants)}
    <div class="cond">
      {#each node.arms as arm, j (j)}
        {#if !reader.hides(sides[j])}
          <div class="arm">
            <Notes notes={arm.notes} />
            <div class="br-h">
              {#if sides[j]}
                {@const side = sides[j]!}
                <span class="chip variant" class:yes={side.side}>{side.side ? side.variant.if_true : side.variant.if_false}</span>
              {:else}
                <span class="kw">{arm.keyword === "elif" ? t("else if") : arm.keyword === "else" ? t("else") : t("if")}</span>
                {#if arm.condition}<Expr expr={arm.condition} />{/if}
              {/if}
              {#if arm.keyword !== "else"}{@render conditionEditor(arm.span.file, arm.span.start, arm.condition)}{/if}
              <Marks marks={arm.marks} />
            </div>
            <div class="branch">
              {#if arm.body.length > 0}<Nodes nodes={arm.body} />{:else}<p class="nothing">{t("nothing")}</p>{/if}
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {:else if node.type === "match"}
    {@const flags = reader.view.flag_functions.match_subjects.includes(node.subject)}
    <div class="cond">
      <div class="match-h">{t("Depends on")} <Expr expr={node.subject} /></div>
      {#each node.arms as arm, j (j)}
        <div class="arm">
          <Notes notes={arm.notes} />
          <div class="br-h">
            <span class="kw">{arm.keyword === "when" ? t("when") : t("otherwise")}</span>
            {#if arm.condition && flags}<FlagChip flag={arm.condition.replace(/^&?["']|["']$/g, "")} />
            {:else if arm.condition}<Expr expr={arm.condition} />{/if}
            {#if arm.keyword === "when"}{@render conditionEditor(arm.span.file, arm.span.start, arm.condition)}{/if}
          </div>
          <div class="branch"><Nodes nodes={arm.body} /></div>
        </div>
      {/each}
    </div>
  {:else if node.type === "while"}
    <div class="cond">
      <div class="br-h"><span class="kw">{t("while")}</span> <Expr expr={node.condition} /></div>
      <div class="branch"><Nodes nodes={node.body} /></div>
    </div>
  {:else if node.type === "random"}
    <div class="cond">
      <div class="match-h">{t("One of these, at random")}</div>
      {#each node.options as option, j (j)}
        <div class="arm">
          {#if option.weight || option.condition}
            <div class="br-h">
              {#if option.weight}<span class="kw">{t("weight {weight}", { weight: option.weight })}</span>{/if}
              {#if option.condition}<span class="when">{t("if")}</span> <Expr expr={option.condition} />{/if}
            </div>
          {/if}
          <div class="branch"><Nodes nodes={option.body} /></div>
        </div>
      {/each}
    </div>
  {:else if node.type === "effect"}
    <div class="fx"><Expr expr={node.expr} /> <Marks marks={node.marks} /></div>
  {:else if node.type === "jump"}
    <JumpView jump={node} />
  {:else if node.type === "anchor" && reader.showIds}
    <div class="anchor tid" id="l-{node.id}">~ {node.id}</div>
  {/if}
{/each}
