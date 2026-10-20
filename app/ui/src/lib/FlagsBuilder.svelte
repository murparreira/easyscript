<script lang="ts">
  // What each flag means, beside the labels that set and read it.
  import { useEditor } from "./editor.svelte";
  import { useReader } from "./reader.svelte";

  const reader = useReader();
  const editor = useEditor()!;

  function save(flag: string, before: string | null, input: HTMLInputElement) {
    const meaning = input.value.trim();
    if (meaning === (before ?? "")) return;
    editor.editProject([{ op: "set_flag_meaning", flag, meaning: meaning || null }]);
  }
</script>

<section class="sec builder" id="b-flags">
  <h2>Flags</h2>
  <p class="lede">What each flag means. Changes go to the project file when you leave the field.</p>
  <div class="tablewrap">
    <table class="flags">
      <thead><tr><th>Flag</th><th>Means</th><th>Set by</th><th>Read by</th></tr></thead>
      <tbody>
        {#each reader.view.flags as flag (flag.flag)}
          <tr>
            <td><code>{flag.flag}</code></td>
            <td>
              <input
                class="meaning"
                value={flag.meaning ?? ""}
                placeholder="what it means"
                aria-label="What {flag.flag} means"
                onchange={(e) => save(flag.flag, flag.meaning, e.currentTarget)}
              />
            </td>
            <td>{flag.set_by.join(", ")}</td>
            <td>{flag.read_by.join(", ")}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>
