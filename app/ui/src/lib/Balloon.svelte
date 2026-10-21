<script lang="ts">
  // A line as a balloon: spoken, or thought; shaking when it shakes. A preview, not the
  // game's own balloon.
  import Face from "./Face.svelte";
  import { t } from "./i18n.svelte";

  let {
    face,
    speaker,
    words,
    thought = false,
    shake = false,
  }: { face: string | null; speaker: string | null; words: string; thought?: boolean; shake?: boolean } = $props();

  // Dialogue Manager's escapes, read as the game shows them.
  const shown = $derived(words.replace(/\\n/g, "\n").replace(/\\([[\]:])/g, "$1"));
  let replay = $state(0);
</script>

<div class="balloon-preview" class:thought>
  <Face image={face} name={speaker} />
  {#key replay}
    <div class="balloon" class:shake>
      {#if speaker}<div class="balloon-who">{speaker}</div>{/if}
      <div class="balloon-text">{shown || "…"}</div>
    </div>
  {/key}
  {#if shake}<button type="button" class="link" onclick={() => replay++}>{t("shake again")}</button>{/if}
</div>
