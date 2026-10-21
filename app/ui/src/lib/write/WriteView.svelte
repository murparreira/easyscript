<script lang="ts">
  // Writing: the scenes on the left, the scene being written in the middle, and playing it
  // on the right.
  import PlayPanel from "../play/PlayPanel.svelte";
  import { useReader } from "../reader.svelte";
  import SceneEditor from "./SceneEditor.svelte";
  import SceneList from "./SceneList.svelte";

  const reader = useReader();
</script>

<div class="write" class:playing={reader.playOpen}>
  <SceneList />
  <SceneEditor />
  {#if reader.playOpen}<PlayPanel />{/if}
</div>
