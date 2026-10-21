<script lang="ts">
  // A list of script steps to read, nested ones drawn inside their menus and branches.
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
  import { t } from "./i18n.svelte";
  import { useReader } from "./reader.svelte";
  import TagChips from "./TagChips.svelte";
  import TextView from "./TextView.svelte";

  let { nodes }: { nodes: Node[] } = $props();
  const reader = useReader();
</script>

{#each nodes as node, i (i)}
  <Notes notes={node.notes} />
  {#if node.type === "line"}
    <LineView line={node} marks={node.marks} />
  {:else if node.type === "menu"}
    <div class="menu" class:thought={node.choices.every((c) => reader.shown(c.tags, "thought").length > 0)}>
      <div class="menu-h">{t("Choices")}</div>
      <ol>
        {#each node.choices as choice, j (j)}
          {@const side = conditionSide(choice.condition, reader.view.variants)}
          {#if !reader.hides(side)}
            <li>
              <Notes notes={choice.notes} />
              <div class="opt-row">
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
              </div>
              {#if choice.body.length > 0}<div class="branch"><Nodes nodes={choice.body} /></div>{/if}
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
