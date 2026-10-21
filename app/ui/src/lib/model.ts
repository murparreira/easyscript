// Pure helpers over the read view's data: places, search text, flags in expressions,
// variants. No state here; see reader.svelte.ts.

import type { Arm } from "./bindings/Arm";
import type { Choice } from "./bindings/Choice";
import type { FlagFunctions } from "./bindings/FlagFunctions";
import type { Label } from "./bindings/Label";
import type { Node } from "./bindings/Node";
import type { Note } from "./bindings/Note";
import type { ReadView } from "./bindings/ReadView";
import type { Segment } from "./bindings/Segment";
import type { VariantView } from "./bindings/VariantView";
import { t } from "./i18n.svelte";
import { normalize } from "./text";

export type SectionOut = { key: string; title: string | null; notes: Note[]; labels: Label[] };
export type FileOut = { path: string; sections: SectionOut[] };
export type PlaceOut = { id: string; name: string; group: string | null; files: FileOut[] };
export type GroupOut = { name: string | null; places: PlaceOut[] };

/** Labels arranged by place, places by group, in the project file's order. Labels in no
 *  place come last, one "place" per file. */
export function organize(view: ReadView): GroupOut[] {
  const places: PlaceOut[] = view.places.map((p) => ({ ...p, files: [] }));
  const byId = new Map(places.map((p) => [p.id, p]));
  const loose: PlaceOut[] = [];
  for (const file of view.script.files) {
    file.sections.forEach((section, s) => {
      for (const label of section.labels) {
        let place = label.place ? byId.get(label.place) : undefined;
        if (!place) {
          place = loose.find((p) => p.id === file.path);
          if (!place) {
            place = { id: file.path, name: file.path, group: null, files: [] };
            loose.push(place);
          }
        }
        let out = place.files.find((f) => f.path === file.path);
        if (!out) {
          out = { path: file.path, sections: [] };
          place.files.push(out);
        }
        const key = `${file.path}#${s}`;
        let sec = out.sections.find((x) => x.key === key);
        if (!sec) {
          // A section's notes go with the first place that shows it.
          const shown = places.some((p) => p.files.some((f) => f.sections.some((x) => x.key === key)));
          sec = { key, title: section.title, notes: shown ? [] : section.notes, labels: [] };
          out.sections.push(sec);
        }
        sec.labels.push(label);
      }
    });
  }
  const groups: GroupOut[] = [];
  for (const place of places.filter((p) => p.files.length > 0)) {
    let group = groups.find((g) => g.name === place.group);
    if (!group) {
      group = { name: place.group, places: [] };
      groups.push(group);
    }
    group.places.push(place);
  }
  if (loose.length > 0) {
    groups.push({ name: places.length > 0 ? t("Not in a place") : null, places: loose });
  }
  return groups;
}

/** The plain text of segments: what a reader sees, without markup. */
export function plain(segments: Segment[]): string {
  return segments
    .map((s) => (s.type === "text" ? s.text : s.type === "random" ? s.options.join(" / ") : s.type === "expr" ? `{{${s.expr}}}` : ""))
    .join("");
}

/** Every node in a list, nested ones included, depth first. */
export function* walk(nodes: Node[]): Generator<Node> {
  for (const node of nodes) {
    yield node;
    switch (node.type) {
      case "menu":
        for (const c of node.choices) yield* walk(c.body);
        break;
      case "branch":
      case "match":
        for (const a of node.arms) yield* walk(a.body);
        break;
      case "while":
        yield* walk(node.body);
        break;
      case "random":
        for (const o of node.options) yield* walk(o.body);
        break;
    }
  }
}

/** What a search looks through for a label, normalized. */
export function labelText(label: Label): string {
  const parts: string[] = [label.id, ...label.notes.map((n) => n.text)];
  const choice = (c: Choice) => parts.push(c.speaker ?? "", plain(c.text), c.condition ?? "");
  for (const node of walk(label.body)) {
    parts.push(...node.notes.map((n) => n.text));
    switch (node.type) {
      case "line":
        parts.push(node.speaker ?? "", plain(node.text));
        break;
      case "menu":
        node.choices.forEach(choice);
        break;
      case "branch":
      case "match":
        node.arms.forEach((a) => parts.push(a.condition ?? ""));
        break;
      case "effect":
        parts.push(node.expr);
        break;
      case "jump":
        if (node.target.kind === "label") parts.push(node.target.value);
        break;
    }
  }
  return normalize(parts.join("\n"));
}

export type ExprPiece = { text: string } | { flag: string; access: "set" | "read" };

/** An expression with flag calls (`has_flag("x")`) picked out. */
export function exprPieces(expr: string, functions: FlagFunctions): ExprPiece[] {
  const names = [...functions.set, ...functions.read].map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  if (names.length === 0) return [{ text: expr }];
  const re = new RegExp(`(?:[A-Za-z_][\\w]*\\.)?\\b(${names.join("|")})\\(\\s*&?["']([^"']*)["']\\s*\\)`, "g");
  const out: ExprPiece[] = [];
  let pos = 0;
  for (const m of expr.matchAll(re)) {
    if (m.index! > pos) out.push({ text: expr.slice(pos, m.index) });
    out.push({ flag: m[2], access: functions.set.includes(m[1]) ? "set" : "read" });
    pos = m.index! + m[0].length;
  }
  if (pos < expr.length) out.push({ text: expr.slice(pos) });
  return out;
}

export type Side = { variant: VariantView; side: boolean };

/** Which side of a variant a condition is on: the condition itself, or its negation. */
export function conditionSide(condition: string | null, variants: VariantView[]): Side | null {
  if (!condition) return null;
  const c = condition.trim();
  for (const variant of variants) {
    if (c === variant.condition) return { variant, side: true };
    const negated = c.replace(/^(not\s+|!\s*)/, "");
    if (negated !== c && negated.trim() === variant.condition) return { variant, side: false };
  }
  return null;
}

/** The variant side of each arm of a branch: an `else` takes the side opposite the `if`. */
export function armSides(arms: Arm[], variants: VariantView[]): Array<Side | null> {
  const first = arms.length > 0 ? conditionSide(arms[0].condition, variants) : null;
  return arms.map((arm, i) => {
    if (i === 0) return first;
    if (arm.keyword === "else" && first && arms.length === 2) return { variant: first.variant, side: !first.side };
    return conditionSide(arm.condition, variants);
  });
}
