<script lang="ts">
  // Every flag: what it means, which labels set it, which read it.
  import { useReader } from "./reader.svelte";

  const reader = useReader();
</script>

<section class="sec" id="flags">
  <h2>Flags</h2>
  <p class="lede">What each flag means, and the labels that set and read it. Flags set or read only by the game's code show no labels here.</p>
  <div class="tablewrap">
    <table class="flags">
      <thead><tr><th>Flag</th><th>Means</th><th>Set by</th><th>Read by</th></tr></thead>
      <tbody>
        {#each reader.view.flags as flag (flag.flag)}
          <tr id="f-{flag.flag}">
            <td><code>{flag.flag}</code></td>
            <td>{flag.meaning ?? ""}</td>
            <td>{#each flag.set_by as l, i (l)}{#if i > 0}, {/if}<a class="lref" href="#l-{l}">{l}</a>{/each}</td>
            <td>{#each flag.read_by as l, i (l)}{#if i > 0}, {/if}<a class="lref" href="#l-{l}">{l}</a>{/each}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>
