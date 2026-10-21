// Playing a script, as the game would: lines one by one, choices to pick, conditions,
// jumps, labels falling through to the next one in their file. Flags and variables set
// while playing are tracked; what only the game knows (its own functions) is asked of the
// writer once and remembered.

import type { Choice } from "../bindings/Choice";
import type { FlagFunctions } from "../bindings/FlagFunctions";
import type { Jump } from "../bindings/Jump";
import type { Label } from "../bindings/Label";
import type { Node } from "../bindings/Node";
import type { ReadView } from "../bindings/ReadView";
import type { Segment } from "../bindings/Segment";
import type { Tag } from "../bindings/Tag";
import type { VariantView } from "../bindings/VariantView";
import { type Known, type Value, change, evaluate, holds, question, shown } from "./expr";

/** What playing knows. Kept between runs until it is reset. */
export class PlayState {
  flags = new Set<string>();
  variables = new Map<string, Value>();
  /** The writer's answers, by question. */
  answers = new Map<string, boolean>();
  /** The arm picked for a `match` subject. */
  picks = new Map<string, number>();
}

/** The script to play: labels by id, and the label after each in its file. */
export type World = {
  labels: Map<string, Label>;
  after: Map<Label, Label | undefined>;
  functions: FlagFunctions;
  variants: VariantView[];
};

export function world(view: ReadView): World {
  const labels = new Map<string, Label>();
  const after = new Map<Label, Label | undefined>();
  for (const file of view.script.files) {
    const all = file.sections.flatMap((s) => s.labels);
    all.forEach((label, i) => {
      if (!labels.has(label.id)) labels.set(label.id, label);
      after.set(label, all[i + 1]);
    });
  }
  return { labels, after, functions: view.flag_functions, variants: view.variants };
}

/** What happened, in order. */
export type Event =
  | { kind: "line"; label: string; speaker: string | null; text: string; tags: Tag[]; concurrent: boolean }
  | { kind: "picked"; speaker: string | null; text: string }
  | { kind: "label"; label: string; how: "start" | "jump" | "return" | "fall" }
  | { kind: "effect"; text: string; applied: boolean }
  | { kind: "answered"; question: string; answer: string };

export type Option = { index: number; speaker: string | null; text: string; tags: Tag[]; condition: string | null; truth: boolean | null };

/** What playing waits for. */
export type Wait =
  | { kind: "next" }
  | { kind: "choose"; options: Option[] }
  /** Does this hold? Answers name the variant's sides when it is one. */
  | { kind: "ask"; question: string; yes: string | null; no: string | null }
  /** Which arm of a `match`? */
  | { kind: "pick"; subject: string; options: string[] }
  | { kind: "end"; why: "end" | "end_conversation" | "ran_out" | "missing" | "expression" | "loop" };

type Frame = { nodes: Node[]; i: number; label: Label; loop?: { condition: string; count: number } };

const MAX_STEPS = 5000;
const MAX_LOOPS = 100;

export class Run {
  events: Event[] = [];
  wait: Wait = { kind: "end", why: "ran_out" };
  #stack: Frame[] = [];
  /** Where `=><` jumps come back to. */
  #returns: Frame[][] = [];
  #known: Known & { setsFlag(name: string): boolean };

  constructor(
    readonly world: World,
    readonly state: PlayState,
  ) {
    const set = new Set(world.functions.set);
    const read = new Set(world.functions.read);
    this.#known = {
      flag: (name) => state.flags.has(name),
      readsFlag: (name) => read.has(name),
      setsFlag: (name) => set.has(name),
      variable: (path) => state.variables.get(path),
      answer: (q) => state.answers.get(q),
    };
  }

  /** Play from the start of a label. */
  start(id: string) {
    this.events = [];
    this.#stack = [];
    this.#returns = [];
    const label = this.world.labels.get(id);
    if (!label) {
      this.wait = { kind: "end", why: "missing" };
      return;
    }
    this.#enter(label, "start");
    this.#go();
  }

  next() {
    if (this.wait.kind === "next") this.#go();
  }

  /** Pick a choice. */
  choose(index: number) {
    if (this.wait.kind !== "choose") return;
    const top = this.#stack[this.#stack.length - 1];
    const menu = top.nodes[top.i] as Extract<Node, { type: "menu" }>;
    const choice: Choice = menu.choices[index];
    top.i++;
    this.events.push({ kind: "picked", speaker: choice.speaker, text: this.#say(choice.text) });
    if (choice.jump) this.#jump(choice.jump);
    else if (choice.body.length > 0) this.#stack.push({ nodes: choice.body, i: 0, label: top.label });
    this.#go();
  }

  /** Answer the question asked. */
  answer(yes: boolean) {
    if (this.wait.kind !== "ask") return;
    const { question: q } = this.wait;
    this.state.answers.set(q, yes);
    this.events.push({ kind: "answered", question: q, answer: (yes ? this.wait.yes : this.wait.no) ?? String(yes) });
    this.#go();
  }

  /** Pick the arm of a `match`. */
  pick(index: number) {
    if (this.wait.kind !== "pick") return;
    this.state.picks.set(this.wait.subject, index);
    this.events.push({ kind: "answered", question: this.wait.subject, answer: this.wait.options[index] });
    this.#go();
  }

  #enter(label: Label, how: "start" | "jump" | "return" | "fall") {
    this.#stack.push({ nodes: label.body, i: 0, label });
    this.events.push({ kind: "label", label: label.id, how });
  }

  #jump(jump: Jump): boolean {
    const target = jump.target;
    switch (target.kind) {
      case "label": {
        const label = this.world.labels.get(target.value);
        if (!label) {
          this.wait = { kind: "end", why: "missing" };
          this.#stack = [];
          return false;
        }
        if (jump.returns) this.#returns.push(this.#stack);
        this.#stack = [];
        this.#enter(label, "jump");
        return true;
      }
      case "end":
        return this.#end("end");
      case "end_conversation":
        this.#returns = [];
        return this.#end("end_conversation");
      case "expression":
        this.#stack = [];
        this.wait = { kind: "end", why: "expression" };
        return false;
    }
  }

  /** The end of a snippet jumped to with `=><` goes back; any other end ends. */
  #end(why: "end" | "end_conversation" | "ran_out"): boolean {
    const back = this.#returns.pop();
    if (back && why !== "end_conversation") {
      this.#stack = back;
      const top = back[back.length - 1];
      if (top) this.events.push({ kind: "label", label: top.label.id, how: "return" });
      return true;
    }
    this.#stack = [];
    this.wait = { kind: "end", why };
    return false;
  }

  /** Ask about a condition: a variant's sides by name. */
  #ask(q: string) {
    const variant = this.world.variants.find((v) => question(v.condition) === q);
    this.wait = { kind: "ask", question: q, yes: variant?.if_true ?? null, no: variant?.if_false ?? null };
  }

  /** Run until there is something to show or decide. */
  #go() {
    for (let steps = 0; steps < MAX_STEPS; steps++) {
      const top = this.#stack[this.#stack.length - 1];
      if (!top) {
        this.wait = { kind: "end", why: "ran_out" };
        return;
      }
      if (top.i >= top.nodes.length) {
        if (top.loop) {
          const again = holds(top.loop.condition, this.#known);
          if (typeof again === "object") return this.#ask(again.ask);
          if (again && ++top.loop.count < MAX_LOOPS) {
            top.i = 0;
            continue;
          }
          if (again) {
            this.wait = { kind: "end", why: "loop" };
            return;
          }
        }
        this.#stack.pop();
        if (this.#stack.length === 0) {
          // A label with no jump at its end goes on into the next one in its file.
          const next = top.loop ? undefined : this.world.after.get(top.label);
          if (next && top.nodes === top.label.body) this.#enter(next, "fall");
          else if (!this.#end("ran_out")) return;
        }
        continue;
      }
      const node = top.nodes[top.i];
      switch (node.type) {
        case "line": {
          top.i++;
          this.events.push({ kind: "line", label: top.label.id, speaker: node.speaker, text: this.#say(node.text), tags: node.tags, concurrent: node.concurrent });
          if (node.jump && !this.#jump(node.jump)) return;
          // Lines said at the same time show together.
          const after = this.#stack[this.#stack.length - 1];
          const following = after?.nodes[after.i];
          if (following?.type === "line" && following.concurrent) continue;
          this.wait = { kind: "next" };
          return;
        }
        case "menu": {
          const options: Option[] = [];
          node.choices.forEach((c, index) => {
            const truth = c.condition ? holds(c.condition, this.#known) : true;
            if (truth === false) return;
            options.push({ index, speaker: c.speaker, text: this.#say(c.text), tags: c.tags, condition: c.condition, truth: typeof truth === "object" ? null : truth });
          });
          if (options.length === 0) {
            top.i++;
            continue;
          }
          this.wait = { kind: "choose", options };
          return;
        }
        case "branch": {
          let taken: Node[] | null = null;
          for (const arm of node.arms) {
            if (arm.keyword === "else" || !arm.condition) {
              taken = arm.body;
              break;
            }
            const truth = holds(arm.condition, this.#known);
            if (typeof truth === "object") return this.#ask(truth.ask);
            if (truth) {
              taken = arm.body;
              break;
            }
          }
          top.i++;
          if (taken) this.#stack.push({ nodes: taken, i: 0, label: top.label });
          continue;
        }
        case "match": {
          const subject = question(node.subject);
          const value = evaluate(node.subject, this.#known);
          let index = -1;
          if ("value" in value) {
            index = node.arms.findIndex((a) => a.keyword === "else" || (a.condition !== null && sameValue(a.condition, value.value, this.#known)));
          } else {
            const picked = this.state.picks.get(subject);
            if (picked === undefined) {
              this.wait = { kind: "pick", subject, options: node.arms.map((a) => (a.keyword === "else" ? "else" : (a.condition ?? ""))) };
              return;
            }
            index = picked;
          }
          top.i++;
          if (index >= 0) this.#stack.push({ nodes: node.arms[index].body, i: 0, label: top.label });
          continue;
        }
        case "while": {
          const truth = holds(node.condition, this.#known);
          if (typeof truth === "object") return this.#ask(truth.ask);
          top.i++;
          if (truth) this.#stack.push({ nodes: node.body, i: 0, label: top.label, loop: { condition: node.condition, count: 0 } });
          continue;
        }
        case "random": {
          const options = node.options.filter((o) => !o.condition || holds(o.condition, this.#known) !== false);
          top.i++;
          const weights = options.map((o) => Math.max(0, Number(o.weight ?? 1)) || 1);
          let roll = Math.random() * weights.reduce((a, b) => a + b, 0);
          const option = options.find((_, i) => (roll -= weights[i]) < 0) ?? options[0];
          if (option) this.#stack.push({ nodes: option.body, i: 0, label: top.label });
          continue;
        }
        case "effect": {
          top.i++;
          this.#apply(node.keyword, node.expr);
          continue;
        }
        case "jump": {
          top.i++;
          if (!this.#jump(node)) return;
          continue;
        }
        case "anchor":
          top.i++;
          continue;
      }
    }
    this.#stack = [];
    this.wait = { kind: "end", why: "loop" };
  }

  #apply(keyword: string, expr: string) {
    const made = change(keyword, expr, this.#known);
    if ("flag" in made) this.state.flags.add(made.flag);
    else if ("variable" in made) this.state.variables.set(made.variable, made.value);
    this.events.push({ kind: "effect", text: `${keyword} ${expr}`, applied: !("unknown" in made) });
  }

  /** A line's words as the game would show them: one random option, values filled in,
   *  inline conditions decided (shown when not known), inline effects applied. */
  #say(segments: Segment[]): string {
    let out = "";
    // Whether each open `[if]` shows its text.
    const showing: boolean[] = [];
    const on = () => showing.every(Boolean);
    for (const s of segments) {
      switch (s.type) {
        case "text":
          if (on()) out += s.text;
          break;
        case "random":
          if (on()) out += s.options[Math.floor(Math.random() * s.options.length)] ?? "";
          break;
        case "expr": {
          if (!on()) break;
          const value = evaluate(s.expr, this.#known);
          out += "value" in value ? shown(value.value) : `{{${s.expr}}}`;
          break;
        }
        case "if": {
          const truth = holds(s.condition, this.#known);
          showing.push(truth !== false);
          break;
        }
        case "else":
          if (showing.length > 0) showing[showing.length - 1] = !showing[showing.length - 1];
          break;
        case "end_if":
          showing.pop();
          break;
        case "effect":
          if (on()) {
            const [keyword, ...rest] = s.expr.trim().split(/\s+/);
            const known = ["do", "do!", "set"].includes(keyword);
            this.#apply(known ? keyword : "do", known ? rest.join(" ") : s.expr);
          }
          break;
        case "markup":
          break;
      }
    }
    return out.replace(/\\n/g, "\n").replace(/\\([[\]:#])/g, "$1").trim();
  }
}

function sameValue(written: string, value: Value, known: Known): boolean {
  const result = evaluate(written, known);
  return "value" in result && result.value === value;
}
