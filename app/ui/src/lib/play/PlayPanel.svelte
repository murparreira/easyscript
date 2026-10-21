<script lang="ts">
  // Play the script beside it, as the game would: from the scene being written, or one
  // chosen. What only the game knows is asked once and remembered in the story state.
  import { tick, untrack } from "svelte";
  import Face from "../Face.svelte";
  import { t } from "../i18n.svelte";
  import { useReader } from "../reader.svelte";
  import { type Event, PlayState, Run, type Wait, world } from "./runner";

  const reader = useReader();
  let story = new PlayState();
  let start = $state(untrack(() => reader.current()?.id ?? ""));
  let run: Run | null = null;
  let events: Event[] = $state.raw([]);
  let wait: Wait = $state.raw({ kind: "end", why: "ran_out" });
  // The view the run started on: when the script changes, restarting plays the new one.
  let played: unknown = $state.raw(null);
  const stale = $derived(played !== null && played !== reader.view);
  let showState = $state(false);
  let log: HTMLDivElement | undefined = $state();
  // Bumped on every step, so the state panel shows the flags as they are now.
  let tickCount = $state(0);

  async function sync() {
    if (!run) return;
    events = [...run.events];
    wait = run.wait;
    tickCount++;
    await tick();
    log?.scrollTo({ top: log.scrollHeight });
  }

  function restart(id = start) {
    if (!id) return;
    start = id;
    run = new Run(world(reader.view), story);
    run.start(id);
    played = reader.view;
    sync();
  }

  // From the scene's "Play from here", and when the panel opens.
  $effect(() => {
    const asked = reader.playFrom;
    untrack(() => restart(asked?.label ?? start));
  });

  // Nothing to play yet (a new project, a scene removed): the scene being written plays.
  $effect(() => {
    const current = reader.current()?.id;
    if (current && !reader.byId.has(untrack(() => start))) untrack(() => restart(current));
  });

  function next() {
    run?.next();
    sync();
  }
  function skip() {
    for (let i = 0; i < 500 && run?.wait.kind === "next"; i++) run.next();
    sync();
  }
  function resetState() {
    story = new PlayState();
    restart();
  }

  function keydown(e: KeyboardEvent) {
    if ((e.key === " " || e.key === "Enter") && wait.kind === "next" && !(e.target instanceof HTMLElement && e.target.closest("input, select, textarea, button"))) {
      e.preventDefault();
      next();
    }
  }

  const places = $derived(
    reader.groups.flatMap((g) => g.places).map((p) => ({ name: p.name, labels: p.files.flatMap((f) => f.sections.flatMap((s) => s.labels.map((l) => l.id))) })),
  );
  const flags = $derived(reader.view.flags.map((f) => f.flag));
  const why = $derived(
    wait.kind !== "end"
      ? ""
      : {
          end: t("The scene ends here."),
          end_conversation: t("The conversation ends here."),
          ran_out: t("Nothing comes after: the conversation ends."),
          missing: t("A jump goes to a scene that does not exist."),
          expression: t("A jump goes where the game's code says; playing stops here."),
          loop: t("This goes round in circles; playing stopped."),
        }[wait.why],
  );
  const known = $derived.by(() => {
    void tickCount;
    return { flags: [...story.flags], answers: [...story.answers], variables: [...story.variables], picks: [...story.picks] };
  });
  function toggleFlag(flag: string, on: boolean) {
    if (on) story.flags.add(flag);
    else story.flags.delete(flag);
    tickCount++;
  }
  function forget(question: string) {
    story.answers.delete(question);
    story.picks.delete(question);
    tickCount++;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside class="play" aria-label={t("Play")} onkeydown={keydown}>
  <div class="play-head">
    <label class="field inline grow">
      <span class="k">{t("Starting scene")}</span>
      <select value={start} onchange={(e) => restart(e.currentTarget.value)}>
        {#each places as place, i (i)}
          <optgroup label={place.name}>
            {#each place.labels as id (id)}<option value={id}>{id}</option>{/each}
          </optgroup>
        {/each}
      </select>
    </label>
    <div class="play-actions">
      <button class="btn small" type="button" onclick={() => restart()}>{t("Restart")}</button>
      <button class="btn small" type="button" onclick={resetState} title={t("Forget the flags set and the answers given")}>{t("Reset story state")}</button>
      <span class="grow"></span>
      <button class="btn small" type="button" aria-pressed={showState} onclick={() => (showState = !showState)}>{t("State")}</button>
      <button class="btn small" type="button" onclick={() => (reader.playOpen = false)} title={t("Close the play panel; Play from here opens it again")}>{t("Hide")}</button>
    </div>
    {#if stale}<p class="hint">{t("The script changed.")} <button type="button" class="link" onclick={() => restart()}>{t("Restart to play the new version")}</button></p>{/if}
  </div>

  {#if showState}
    <div class="play-state">
      <div class="k">{t("Flags")}</div>
      {#if flags.length === 0}<p class="hint">{t("The project has no flags.")}</p>{/if}
      <div class="flag-list">
        {#each flags as flag (flag)}
          <label class="tog small"><input type="checkbox" checked={known.flags.includes(flag)} onchange={(e) => toggleFlag(flag, e.currentTarget.checked)} /> {flag}</label>
        {/each}
      </div>
      {#if known.variables.length > 0}
        <div class="k">{t("Variables")}</div>
        {#each known.variables as [name, value] (name)}<div class="tid">{name} = {JSON.stringify(value)}</div>{/each}
      {/if}
      {#if known.answers.length > 0 || known.picks.length > 0}
        <div class="k">{t("Your answers")}</div>
        {#each known.answers as [question, answer] (question)}
          <div class="answer"><code>{question}</code> → {answer ? t("yes") : t("no")} <button type="button" class="link" onclick={() => forget(question)}>{t("forget")}</button></div>
        {/each}
        {#each known.picks as [subject, index] (subject)}
          <div class="answer"><code>{subject}</code> → #{index + 1} <button type="button" class="link" onclick={() => forget(subject)}>{t("forget")}</button></div>
        {/each}
      {/if}
    </div>
  {/if}

  <div class="play-log" bind:this={log} aria-live="polite">
    {#each events as event, i (i)}
      {#if event.kind === "label"}
        <div class="play-scene">
          {event.how === "start" ? t("Scene {scene}", { scene: event.label }) : event.how === "fall" ? t("On into {scene}", { scene: event.label }) : event.how === "return" ? t("Back in {scene}", { scene: event.label }) : t("To {scene}", { scene: event.label })}
        </div>
      {:else if event.kind === "line"}
        {@const thought = reader.shown(event.tags, "thought").length > 0}
        <div class="bubble-row" class:concurrent={event.concurrent}>
          <Face image={reader.face(event.speaker, event.tags)} name={event.speaker} />
          <div class="bubble" class:thought class:last={i === events.length - 1}>
            {#if event.speaker}<div class="bubble-who">{event.speaker}</div>{/if}
            <div class="bubble-text">{event.text}</div>
          </div>
        </div>
      {:else if event.kind === "picked"}
        <div class="bubble-row mine"><div class="bubble picked">▸ {event.text}</div></div>
      {:else if event.kind === "effect"}
        <div class="play-note" class:applied={event.applied}>⚙ {event.text}</div>
      {:else if event.kind === "answered"}
        <div class="play-note">{t("{question}: {answer}", { question: event.question, answer: event.answer === "true" ? t("yes") : event.answer === "false" ? t("no") : event.answer })}</div>
      {/if}
    {/each}
  </div>

  <div class="play-foot">
    {#if wait.kind === "next"}
      <div class="play-actions">
        <button class="btn primary" type="button" onclick={next}>{t("Next ▸")}</button>
        <button class="btn" type="button" onclick={skip}>{t("Skip to next choice")}</button>
      </div>
      <p class="hint">{t("Space or Enter also moves on.")}</p>
    {:else if wait.kind === "choose"}
      <div class="play-choices">
        {#each wait.options as option (option.index)}
          <button class="choice-btn" type="button" onclick={() => { run?.choose(option.index); sync(); }}>
            ▸ {option.text}
            {#if option.truth === null}<span class="hint" title={option.condition ?? ""}> · {t("if {condition}", { condition: option.condition ?? "" })}</span>{/if}
          </button>
        {/each}
      </div>
    {:else if wait.kind === "ask"}
      <div class="play-ask">
        <p>{wait.yes ? t("Which one?") : t("Does this hold now?")} <code>{wait.question}</code></p>
        <div class="play-actions">
          <button class="btn primary" type="button" onclick={() => { run?.answer(true); sync(); }}>{wait.yes ?? t("Yes")}</button>
          <button class="btn" type="button" onclick={() => { run?.answer(false); sync(); }}>{wait.no ?? t("No")}</button>
        </div>
        <p class="hint">{t("Only the game knows; your answer is remembered until you reset the story state.")}</p>
      </div>
    {:else if wait.kind === "pick"}
      {@const asked = wait}
      <div class="play-ask">
        <p>{t("Which one is it?")} <code>{asked.subject}</code></p>
        <div class="play-choices">
          {#each asked.options as option, i (i)}<button class="choice-btn" type="button" onclick={() => { run?.pick(i); sync(); }}>{option}</button>{/each}
        </div>
      </div>
    {:else}
      {#if played}
        <p class="play-end">{why}</p>
        <div class="play-actions"><button class="btn primary" type="button" onclick={() => restart()}>{t("Play again")}</button></div>
      {:else}
        <p class="play-end">{t("Nothing to play yet: write a scene first.")}</p>
      {/if}
    {/if}
  </div>
</aside>
