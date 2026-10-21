<script lang="ts">
  // The scene being written: where it plays, what is noted about it, its lines and choices
  // to change, add to and move, and what comes after it.
  import { ask } from "@tauri-apps/plugin-dialog";
  import { useEditor } from "../editor.svelte";
  import FlagChip from "../FlagChip.svelte";
  import { plural, t } from "../i18n.svelte";
  import LineEditor from "../LineEditor.svelte";
  import Marks from "../Marks.svelte";
  import { fileName, walk } from "../model";
  import Notes from "../Notes.svelte";
  import { useReader } from "../reader.svelte";
  import { ago, sceneKey, useTeam } from "../team.svelte";
  import AddBar from "./AddBar.svelte";
  import NewFile from "./NewFile.svelte";
  import Steps from "./Steps.svelte";

  const reader = useReader();
  const editor = useEditor()!;
  const team = useTeam();

  const label = $derived(reader.current());
  const file = $derived(label ? reader.view.script.files[label.span.file] : undefined);
  const section = $derived(file?.sections.find((s) => label && s.labels.includes(label)));
  const place = $derived(label?.place ? reader.view.places.find((p) => p.id === label.place) : undefined);
  const counts = $derived.by(() => {
    let lines = 0;
    let choices = 0;
    for (const node of walk(label?.body ?? [])) {
      if (node.type === "line") lines++;
      else if (node.type === "menu") choices += node.choices.length;
    }
    return { lines, choices };
  });
  const sets = $derived(reader.view.flags.filter((f) => label && f.set_by.includes(label.id)));
  const reads = $derived(reader.view.flags.filter((f) => label && f.read_by.includes(label.id)));
  const entry = $derived(label ? reader.view.flow.entries.includes(`${label.span.file}:${label.id}`) : false);
  const next = $derived.by(() => {
    if (!file || !label) return undefined;
    const all = file.sections.flatMap((s) => s.labels);
    return all[all.indexOf(label) + 1];
  });
  const last = $derived(label?.body[label.body.length - 1]);
  // Where lines added at the end go: before a last jump (`=> END`), else after the last line.
  const end = $derived.by(() => {
    if (!label) return null;
    if (!last) return { at: label.span.start, mode: "after" as const };
    if (last.type === "jump") return { at: last.span.start, mode: "before" as const };
    // After the last choice or arm: after the first would land inside the block.
    const tail =
      last.type === "menu" ? last.choices[last.choices.length - 1].span : last.type === "branch" || last.type === "match" ? last.arms[last.arms.length - 1].span : last.type === "random" ? last.options[last.options.length - 1].span : last.span;
    return { at: tail.start, mode: "after" as const };
  });
  const goesOn = $derived(!last || last.type !== "jump");
  const comesBack = $derived(reader.view.flow.edges.filter((e) => label && e.to === `${label.span.file}:${label.id}` && e.kind !== "fall_through").length);

  // What the team did to this scene: changed it on origin (not here yet), is changing it on
  // a branch, changed it since the writer last looked.
  const teamKey = $derived(label && file ? sceneKey(file.path, label.id) : "");
  const incoming = $derived(team?.incoming.get(teamKey) ?? []);
  const others = $derived(team?.others.get(teamKey) ?? []);
  const since = $derived(team?.since.get(teamKey) ?? []);
  const who = (xs: { author: string }[]) => [...new Set(xs.map((x) => x.author))].join(", ");
  let clash: string | null = $state(null);
  async function bringIn() {
    const done = await team?.bringIn();
    clash = done && done.conflicts.length > 0 ? t("You and the team changed the same lines in {files}, so nothing was brought in. Commit your changes and settle it in git, or ask a teammate.", { files: done.conflicts.join(", ") }) : null;
  }

  async function remove() {
    if (!label || !file) return;
    const warning = entry ? ` ${t("The game starts this scene by its name: remove it from the game's code too.")}` : "";
    const sure = await ask(t("Remove the scene {scene} and its lines?", { scene: label.id }) + warning, { title: t("Remove scene"), kind: "warning" });
    if (!sure) return;
    const above = [...label.notes, ...label.marks].map((n) => n.span.start).filter((s) => s < label.span.start);
    const from = above.length > 0 ? Math.min(...above) : null;
    const all = file.sections.flatMap((s) => s.labels);
    const neighbour = all[all.indexOf(label) - 1] ?? all[all.indexOf(label) + 1];
    if ((await editor.edit(file.path, [{ op: "remove_label", at: label.span.start, from }])) !== null) reader.scene = neighbour?.id ?? null;
  }

  function play() {
    if (!label) return;
    reader.playOpen = true;
    reader.playFrom = { label: label.id };
  }
</script>

{#if label && file}
  <section class="scene" aria-label={label.id}>
    {#key label.id + label.span.file}
      <header class="scene-head">
        <div class="eyebrow">
          {place?.name ?? fileName(file.path)}{#if reader.showIds}<span class="path">&ensp;{file.path}</span>{/if}
        </div>
        <h1 class="scene-title">{label.id}</h1>
        <div class="scene-meta">
          {#if entry}<span class="chip entry" title={t("The game's code starts this scene by name")}>▶ {t("The game starts it")}</span>{/if}
          <span class="chip quiet">{plural(counts.lines, "{n} line", "{n} lines")}</span>
          {#if counts.choices > 0}<span class="chip quiet">{plural(counts.choices, "{n} choice", "{n} choices")}</span>{/if}
          {#if comesBack > 0}<span class="chip quiet" title={t("Jumps and choices that lead here")}>{plural(comesBack, "{n} way in", "{n} ways in")}</span>{/if}
          <Marks marks={label.marks} />
        </div>
        <div class="scene-actions">
          <button class="btn primary small" type="button" onclick={play}>{t("▶ Play from here")}</button>
          <button class="btn small" type="button" onclick={() => { reader.mapFocus = label.id; reader.tab = "map"; }}>{t("Show on map")}</button>
          <span class="grow"></span>
          <button class="btn small danger" type="button" onclick={remove} disabled={editor.busy}>{t("Remove scene")}</button>
        </div>
        {#if sets.length > 0 || reads.length > 0}
          <div class="scene-flags">
            {#if reads.length > 0}<span class="k">{t("Reads")}</span>{#each reads as f (f.flag)}<FlagChip flag={f.flag} />{/each}{/if}
            {#if sets.length > 0}<span class="k">{t("Sets")}</span>{#each sets as f (f.flag)}<FlagChip flag={f.flag} set />{/each}{/if}
          </div>
        {/if}
      </header>

      {#if incoming.length > 0}
        <div class="banner in">
          <p>{t("{who} changed this scene on origin, and your copy does not have it yet. Bring the team's changes in before you write here, so you write on the latest.", { who: who(incoming) })}</p>
          <button class="btn small primary" type="button" onclick={bringIn} disabled={team?.busy || editor.busy}>{team?.busy ? t("Bringing in…") : t("Bring them in")}</button>
        </div>
      {/if}
      {#if clash}<p class="error">{clash}</p>{/if}
      {#each others as work (work.branch)}
        <div class="banner other">
          <p>{t("{who} is also changing this scene, on the branch {branch} ({when}): “{summary}”. Talk before you both change the same lines.", { who: work.last.mine ? t("You") : work.last.author, branch: work.branch, when: ago(work.last.time), summary: work.last.summary })}</p>
        </div>
      {/each}
      {#if since.length > 0}
        <div class="banner new">
          <p>{t("Changed by {who} since you last looked: “{summary}” ({when}).", { who: who(since), summary: since[0].summary, when: ago(since[0].time) })}</p>
          <button type="button" class="link" onclick={() => team?.seeScene(teamKey)}>{t("Got it")}</button>
        </div>
      {/if}

      {#if section && (section.title || section.notes.length > 0)}
        <div class="scene-note group">
          <div class="k">{section.title ? t("Part: {title}", { title: section.title }) : t("About this part")}</div>
          <Notes notes={section.notes} />
        </div>
      {/if}
      {#if label.notes.length > 0}
        <div class="scene-note">
          <div class="k">{t("Scene note")}</div>
          <Notes notes={label.notes} />
        </div>
      {/if}

      <div class="scene-body">
        {#if label.body.length === 0 && !editor.is(label.span.file, label.span.start, "after")}
          <p class="nothing">{t("This scene has no lines yet.")}</p>
        {/if}
        <Steps nodes={label.body} />
        {#if editor.target && editor.is(label.span.file, label.span.start, "after") && label.body.length === 0}
          <LineEditor target={editor.target} />
        {/if}
        {#if end}
          <AddBar file={label.span.file} at={end.at} mode={end.mode} />
        {/if}
        {#if goesOn}
          <p class="after">
            {#if next}{t("Then goes on into")} <button type="button" class="link" onclick={() => (reader.scene = next.id)}>{next.id}</button>{:else}{t("Then the conversation ends.")}{/if}
          </p>
        {/if}
      </div>
    {/key}
  </section>
{:else}
  <section class="scene">
    <div class="first-steps">
      <div class="eyebrow">{t("A new script")}</div>
      <h1 class="scene-title">{t("No dialogue here yet")}</h1>
      <p class="lede">{t("Dialogue lives in .dialogue files in the game's folder, as scenes: a scene is what a character says when the game starts it by name. Make the first file and its first scene, then write its lines here and play them on the right.")}</p>
      <NewFile done={() => {}} first />
    </div>
  </section>
{/if}
