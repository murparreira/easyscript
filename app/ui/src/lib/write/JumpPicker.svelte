<script lang="ts">
  // Where to go next: nowhere (on to the next line), a scene, or the end. Scenes by place.
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";

  let { value = $bindable(), none = true, label }: { value: string; none?: boolean; label?: string } = $props();
  const reader = useReader();
  const places = $derived(
    reader.groups.flatMap((g) => g.places).map((p) => ({ name: p.name, labels: p.files.flatMap((f) => f.sections.flatMap((s) => s.labels.map((l) => l.id))) })),
  );
</script>

<label class="field inline">
  <span class="k">{label ?? t("Then")}</span>
  <select bind:value>
    {#if none}<option value="">{t("go on")}</option>{/if}
    <option value="END">{t("end the scene")}</option>
    <option value="END!">{t("end the conversation")}</option>
    {#each places as place, i (i)}
      <optgroup label={place.name}>
        {#each place.labels as id (id)}<option value={id}>{t("go to {scene}", { scene: id })}</option>{/each}
      </optgroup>
    {/each}
  </select>
</label>
