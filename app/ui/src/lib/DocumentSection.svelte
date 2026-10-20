<script lang="ts">
  // A Markdown document from the project; `[[label]]` links into the script.
  import { marked } from "marked";
  import type { DocumentView } from "./bindings/DocumentView";
  import { useReader } from "./reader.svelte";

  let { doc, index }: { doc: DocumentView; index: number } = $props();
  const reader = useReader();

  const escape = (s: string) => s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
  const html = $derived.by(() => {
    const linked = doc.text.replace(/\[\[([^[\]|]+?)\]\]/g, (_, id: string) => {
      const label = id.trim();
      const cls = reader.labelIds.has(label) ? "lref" : "lref broken";
      return `<a class="${cls}" href="#l-${escape(label)}">${escape(label)}</a>`;
    });
    return marked.parse(linked, { async: false, gfm: true });
  });
</script>

<section class="sec doc" id="d-{index}">
  <!-- The project's own Markdown, written by its team. -->
  {@html html}
  {#if reader.showIds}<div class="pmeta">{doc.path}</div>{/if}
</section>
