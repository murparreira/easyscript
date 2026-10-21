<script lang="ts">
  // Writing: the scenes on the left, the scene being written in the middle, and playing it
  // on the right.
  import PlayPanel from "../play/PlayPanel.svelte";
  import { PLAY_WIDTH, RAIL_WIDTH, useReader } from "../reader.svelte";
  import Splitter from "../Splitter.svelte";
  import SceneEditor from "./SceneEditor.svelte";
  import SceneList from "./SceneList.svelte";

  const reader = useReader();
</script>

<div class="write" class:playing={reader.playOpen} style:--rail-w="{reader.railWidth}px" style:--play-w="{reader.playWidth}px">
  <div class="side-wrap">
    <SceneList />
    <Splitter bind:width={reader.railWidth} edge="right" min={180} max={520} initial={RAIL_WIDTH} />
  </div>
  <SceneEditor />
  {#if reader.playOpen}
    <div class="side-wrap">
      <PlayPanel />
      <Splitter bind:width={reader.playWidth} edge="left" min={260} max={720} initial={PLAY_WIDTH} />
    </div>
  {/if}
</div>
