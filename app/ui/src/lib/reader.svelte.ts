// The reader's state: the view being read, what the reader chose (search, variant filters,
// toggles), and indexes derived from the view. Shared with every component by context.

import { getContext, setContext } from "svelte";
import type { CharacterView } from "./bindings/CharacterView";
import type { Label } from "./bindings/Label";
import type { ReadView } from "./bindings/ReadView";
import type { Tag } from "./bindings/Tag";
import type { TagView } from "./bindings/TagView";
import { type GroupOut, type Side, labelText, organize, walk } from "./model";
import { normalize } from "./text";

export type VariantChoice = "all" | "true" | "false";
/** What the app shows: writing a scene, the story map, the whole script to read, or (in
 *  the app) the project file. */
export type Tab = "write" | "map" | "read" | "project";
export type Theme = "system" | "light" | "dark";

const PREFS = "easyscript:reader";

type Prefs = {
  variants: Record<string, VariantChoice>;
  showIds: boolean;
  showMarks: boolean;
  theme: Theme;
  playOpen: boolean;
};

function loadPrefs(): Partial<Prefs> {
  try {
    return JSON.parse(localStorage.getItem(PREFS) ?? "{}") as Partial<Prefs>;
  } catch {
    return {};
  }
}

export class Reader {
  view: ReadView;
  query = $state("");
  variants: Record<string, VariantChoice> = $state({});
  showIds = $state(true);
  showMarks = $state(false);
  theme: Theme = $state("system");
  /** The play panel beside the scene being written. */
  playOpen = $state(true);
  /** Editing is possible (the desktop app); the static page reads. */
  writable: boolean;
  tab: Tab = $state("read");
  /** The scene being written: a label's id. */
  scene: string | null = $state(null);
  /** Labels by id (the first, if two files have the same). */
  byId: Map<string, Label>;
  /** A label for the map to point at when it opens. */
  mapFocus: string | null = $state(null);
  /** Asked to play from a label (a new object each time, so asking again plays again). */
  playFrom: { label: string } | null = $state(null);

  /** The search, normalized; searching starts at two characters. */
  q: string;
  searching: boolean;
  groups: GroupOut[];
  labelTexts: Map<Label, string>;
  labelIds: Set<string>;
  characters: Map<string, CharacterView>;
  tagSpecs: Map<string, TagView>;
  /** A CSS class per image, so each data URI is in the page once. */
  imageClasses: Map<string, string>;
  imageCss: string;
  /** Speaker names to suggest: the characters' first, then any others in the script. */
  speakers: string[];
  /** For each speaker, the faces their lines use, most used first. */
  facesBySpeaker: Map<string, string[]>;
  /** How the script calls a flag in a condition, with `{flag}` for its name. */
  flagCall: string | null;
  /** How the script sets a flag (`do Globals.set_flag("{flag}")`), with `{flag}` for its name. */
  flagSet: string | null;

  constructor(view: ReadView, writable = false) {
    this.view = $state.raw(view);
    this.writable = writable;
    this.tab = writable ? "write" : "read";
    this.q = $derived(normalize(this.query.trim()));
    this.searching = $derived(this.q.length > 1);
    this.groups = $derived(organize(this.view));
    this.labelTexts = $derived(new Map(this.allLabels().map((l) => [l, labelText(l)])));
    this.labelIds = $derived(new Set(this.allLabels().map((l) => l.id)));
    this.byId = $derived(new Map(this.allLabels().reverse().map((l) => [l.id, l])));
    this.characters = $derived(new Map(this.view.characters.flatMap((c) => c.speakers.map((s) => [s, c] as const))));
    this.tagSpecs = $derived(new Map(this.view.tags.map((t) => [t.name, t])));
    this.imageClasses = $derived(new Map(Object.keys(this.view.images).map((key, i) => [key, `img-${i}`])));
    this.imageCss = $derived(
      Object.entries(this.view.images)
        .map(([key, uri]) => `.${this.imageClasses.get(key)}{background-image:url("${uri}")}`)
        .join("\n"),
    );
    this.speakers = $derived([
      ...new Set([...this.view.characters.flatMap((c) => c.speakers), ...this.view.stats.speakers.map((s) => s.name).filter(Boolean)]),
    ]);
    this.facesBySpeaker = $derived(facesBySpeaker(this.view, this.tagSpecs));
    this.flagCall = $derived(flagCall(this.view));
    this.flagSet = $derived(flagSet(this.view));
    const prefs = loadPrefs();
    this.variants = prefs.variants ?? {};
    this.showIds = prefs.showIds ?? true;
    this.showMarks = prefs.showMarks ?? false;
    this.theme = prefs.theme ?? "system";
    this.playOpen = prefs.playOpen ?? true;
    $effect.root(() => {
      $effect(() => {
        const prefs: Prefs = {
          variants: this.variants,
          showIds: this.showIds,
          showMarks: this.showMarks,
          theme: this.theme,
          playOpen: this.playOpen,
        };
        try {
          localStorage.setItem(PREFS, JSON.stringify(prefs));
        } catch {
          // Private windows and blocked storage: the reader still works, without memory.
        }
      });
    });
  }

  allLabels(): Label[] {
    return this.view.script.files.flatMap((f) => f.sections.flatMap((s) => s.labels));
  }

  shows(label: Label): boolean {
    return !this.searching || (this.labelTexts.get(label) ?? "").includes(this.q);
  }

  /** Is this side of a variant hidden by the reader's filter? */
  hides(side: Side | null): boolean {
    if (!side) return false;
    const choice = this.variants[side.variant.name] ?? "all";
    return choice !== "all" && (choice === "true") !== side.side;
  }

  tagSpec(name: string): TagView | undefined {
    return this.tagSpecs.get(name);
  }

  /** A line's face: its face tag's image, else its speaker's portrait. A CSS class. */
  face(speaker: string | null, tags: Tag[]): string | null {
    for (const tag of tags) {
      const spec = this.tagSpecs.get(tag.name);
      if (spec?.show === "face" && spec.path && tag.value) {
        const cls = this.imageClasses.get(spec.path.replace("{value}", tag.value));
        if (cls) return cls;
      }
    }
    const portrait = speaker ? this.characters.get(speaker)?.portrait : null;
    return portrait ? (this.imageClasses.get(portrait) ?? null) : null;
  }

  /** The scene being written, or the first one. */
  current(): Label | undefined {
    const first = this.groups[0]?.places[0]?.files[0]?.sections.find((s) => s.labels.length > 0)?.labels[0];
    return (this.scene ? this.byId.get(this.scene) : undefined) ?? first ?? this.allLabels()[0];
  }

  /** Show a label: to write it, in the app; in the whole script, in the static page or
   *  when reading. */
  goTo(label: string) {
    if (this.writable && this.tab !== "read") {
      this.tab = "write";
      this.scene = label;
      return;
    }
    this.tab = "read";
    setTimeout(() => {
      location.hash = `l-${label}`;
      document.getElementById(`l-${label}`)?.scrollIntoView({ block: "start", behavior: "instant" });
    }, 0);
  }

  /** Tags shown a certain way (`thought`, `shake`, `voice`). */
  shown(tags: Tag[], show: string): Tag[] {
    return tags.filter((t) => this.tagSpecs.get(t.name)?.show === show);
  }
}

/** The faces each speaker's lines use, most used first. */
function facesBySpeaker(view: ReadView, specs: Map<string, TagView>): Map<string, string[]> {
  const counts = new Map<string, Map<string, number>>();
  for (const file of view.script.files) {
    for (const label of file.sections.flatMap((s) => s.labels)) {
      for (const node of walk(label.body)) {
        const lines = node.type === "line" ? [node] : node.type === "menu" ? node.choices : [];
        for (const line of lines) {
          for (const tag of line.tags) {
            if (specs.get(tag.name)?.show !== "face" || !tag.value) continue;
            const speaker = line.speaker ?? "";
            const faces = counts.get(speaker) ?? new Map<string, number>();
            faces.set(tag.value, (faces.get(tag.value) ?? 0) + 1);
            counts.set(speaker, faces);
          }
        }
      }
    }
  }
  return new Map([...counts].map(([speaker, faces]) => [speaker, [...faces].sort((a, b) => b[1] - a[1]).map(([f]) => f)]));
}

/** The way the script writes a flag check, taken from a condition that has one. */
function flagCall(view: ReadView): string | null {
  const read = view.flag_functions.read[0];
  if (!read) return null;
  const re = new RegExp(`([A-Za-z_][\\w.]*\\.)?${read.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\(`);
  for (const file of view.script.files) {
    for (const label of file.sections.flatMap((s) => s.labels)) {
      for (const node of walk(label.body)) {
        const conditions = [
          ...(node.type === "branch" || node.type === "match" ? node.arms.map((a) => a.condition) : []),
          ...(node.type === "menu" ? node.choices.map((c) => c.condition) : []),
        ];
        for (const condition of conditions) {
          const m = condition?.match(re);
          if (m) return `${m[1] ?? ""}${read}("{flag}")`;
        }
      }
    }
  }
  return `${read}("{flag}")`;
}

/** The way the script sets a flag, taken from an effect that does. */
function flagSet(view: ReadView): string | null {
  const set = view.flag_functions.set[0];
  if (!set) return null;
  const re = new RegExp(`([A-Za-z_][\\w.]*\\.)?${set.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\(`);
  for (const file of view.script.files) {
    for (const label of file.sections.flatMap((s) => s.labels)) {
      for (const node of walk(label.body)) {
        if (node.type !== "effect") continue;
        const m = node.expr.match(re);
        if (m) return `${node.keyword} ${m[1] ?? ""}${set}("{flag}")`;
      }
    }
  }
  return `do ${set}("{flag}")`;
}

const KEY = Symbol("reader");

export function provideReader(reader: Reader) {
  setContext(KEY, reader);
}

export function useReader(): Reader {
  return getContext<Reader>(KEY);
}
