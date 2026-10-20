<script lang="ts">
  // The text of a line or choice: words, random options, expressions, inline conditions.
  import type { Segment } from "./bindings/Segment";
  import Expr from "./Expr.svelte";
  import Hl from "./Hl.svelte";
  import { useReader } from "./reader.svelte";
  import { parentheticals } from "./text";

  let { segments }: { segments: Segment[] } = $props();
  const reader = useReader();
</script>

{#each segments as s, i (i)}{#if s.type === "text"}{#each parentheticals(s.text) as p, j (j)}{#if p.paren}<span
          class="paren"><Hl text={p.text} /></span
        >{:else}<Hl text={p.text} />{/if}{/each}{:else if s.type === "random"}<span
      class="rand"
      title="One of these, at random">{#each s.options as option, j (j)}{#if j > 0}<span class="sep"> / </span>{/if}<Hl
          text={option} />{/each}</span
    >{:else if s.type === "expr"}<code class="expr">{"{{"}<Hl text={s.expr} />{"}}"}</code>{:else if s.type === "if"}<span
      class="inline-if"><span class="k">if</span> <Expr expr={s.condition} /></span
    >{:else if s.type === "else"}<span class="inline-if"><span class="k">else</span></span>{:else if s.type === "end_if"}<span
      class="inline-if"><span class="k">end if</span></span
    >{:else if s.type === "effect"}<span class="inline-if"><span class="k">does</span> <Expr expr={s.expr} /></span
    >{:else if s.type === "markup" && reader.showIds}<code class="markup">[{s.name}{s.args ? ` ${s.args}` : ""}]</code>{/if}{/each}
