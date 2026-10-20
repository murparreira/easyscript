<script lang="ts">
  // Variants: conditions the reader can filter the script by.
  import ConditionInput from "./ConditionInput.svelte";
  import { useEditor } from "./editor.svelte";
  import { useReader } from "./reader.svelte";

  const reader = useReader();
  const editor = useEditor()!;
  type Form = { name: string; condition: string; yes: string; no: string; isNew: boolean };
  let form: Form | null = $state(null);

  async function save() {
    if (!form) return;
    const ok = await editor.editProject([{ op: "set_variant", name: form.name, condition: form.condition, if_true: form.yes, if_false: form.no }]);
    if (ok) form = null;
  }
</script>

<section class="sec builder" id="b-variants">
  <h2>Variants</h2>
  <p class="lede">A condition the script branches on (a game mode, a difficulty), with a name for each side; the reader can show one side only.</p>
  <div class="b-actions"><button class="btn" type="button" onclick={() => (form = { name: "", condition: "", yes: "", no: "", isNew: true })}>Add a variant</button></div>
  {#if form?.isNew}{@render editForm()}{/if}
  <ol class="cards">
    {#each reader.view.variants as v (v.name)}
      <li class="card">
        {#if form && !form.isNew && form.name === v.name}
          {@render editForm()}
        {:else}
          <div class="card-head">
            <div>
              <h4>{v.name}</h4>
              <div><code>{v.condition}</code>: <span class="chip variant yes">{v.if_true}</span> / <span class="chip variant">{v.if_false}</span></div>
            </div>
            <div class="card-tools">
              <button type="button" title="Edit" onclick={() => (form = { name: v.name, condition: v.condition, yes: v.if_true, no: v.if_false, isNew: false })}>✎</button>
              <button type="button" title="Remove" onclick={() => editor.editProject([{ op: "remove_variant", name: v.name }])}>✕</button>
            </div>
          </div>
        {/if}
      </li>
    {/each}
  </ol>
</section>

{#snippet editForm()}
  {#if form}
    <form class="line-editor" onsubmit={(e) => { e.preventDefault(); save(); }}>
      <div class="le-fields">
        {#if form.isNew}<label class="le-field"><span class="k">Name</span><input bind:value={form.name} placeholder="Mode" /></label>{/if}
        <ConditionInput bind:value={form.condition} label="Condition, as the script writes it" />
        <div class="le-row">
          <label class="le-field"><span class="k">When it holds</span><input bind:value={form.yes} placeholder="Easy" /></label>
          <label class="le-field"><span class="k">When it does not</span><input bind:value={form.no} placeholder="Hard" /></label>
        </div>
      </div>
      {#if editor.error}<p class="error">{editor.error}</p>{/if}
      <div class="le-actions">
        <button class="btn primary" type="submit" disabled={editor.busy}>{form.isNew ? "Add" : "Apply"}</button>
        <button class="btn" type="button" onclick={() => (form = null)}>Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}
