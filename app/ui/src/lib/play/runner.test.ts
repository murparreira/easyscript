import { describe, expect, it } from "vitest";
import type { Label } from "../bindings/Label";
import type { Node } from "../bindings/Node";
import { type Known, evaluate, holds } from "./expr";
import { PlayState, Run, type World } from "./runner";

const span = { file: 0, start: 0, end: 0 };
const base = { span, notes: [], marks: [] };
const say = (speaker: string | null, text: string, extra: Partial<Extract<Node, { type: "line" }>> = {}): Node => ({
  ...base,
  type: "line",
  speaker,
  text: [{ type: "text", text }],
  words: text,
  tags: [],
  jump: null,
  id: null,
  concurrent: false,
  ...extra,
});
const to = (label: string, returns = false) => ({ target: { kind: "label" as const, value: label }, returns, span });
const end = { target: { kind: "end" as const }, returns: false, span };
const jump = (j: typeof end | ReturnType<typeof to>): Node => ({ ...base, type: "jump", ...j });
const choice = (text: string, body: Node[], condition: string | null = null, j: typeof end | ReturnType<typeof to> | null = null) => ({
  ...base,
  speaker: null,
  text: [{ type: "text" as const, text }],
  words: text,
  tags: [],
  condition,
  jump: j,
  id: null,
  body,
});
const label = (id: string, body: Node[]): Label => ({ id, span, place: null, notes: [], marks: [], body });

function play(labels: Label[]): World {
  const after = new Map<Label, Label | undefined>();
  labels.forEach((l, i) => after.set(l, labels[i + 1]));
  return {
    labels: new Map(labels.map((l) => [l.id, l])),
    after,
    functions: { set: ["set_flag"], read: ["has_flag"], match_subjects: [] },
    variants: [{ name: "Mode", condition: "Globals.easy()", if_true: "Easy", if_false: "Hard" }],
  };
}

const lines = (run: Run) => run.events.flatMap((e) => (e.kind === "line" ? [`${e.speaker ?? "-"}: ${e.text}`] : e.kind === "picked" ? [`> ${e.text}`] : []));

describe("expressions", () => {
  const known = (answers: Record<string, boolean> = {}, flags: string[] = [], variables: Record<string, number> = {}): Known => ({
    flag: (f) => flags.includes(f),
    readsFlag: (n) => n === "has_flag",
    variable: (p) => variables[p],
    answer: (q) => answers[q],
  });

  it("reads flags, variables and literals", () => {
    expect(holds('Globals.has_flag("a")', known({}, ["a"]))).toBe(true);
    expect(holds('not has_flag("a") and has_flag("b")', known({}, ["b"]))).toBe(true);
    expect(holds("gold >= 3", known({}, [], { gold: 4 }))).toBe(true);
    expect(evaluate('"a" + "b"', known())).toEqual({ value: "ab" });
  });

  it("asks the smallest part that decides", () => {
    expect(holds('Globals.has_item("coffee") and has_flag("x")', known({}, ["x"]))).toEqual({ ask: 'Globals.has_item("coffee")' });
    // A false side decides without asking.
    expect(holds('Globals.has_item("coffee") and has_flag("x")', known())).toBe(false);
    expect(holds("Globals.level() == 0", known())).toEqual({ ask: "Globals.level() == 0" });
    expect(holds("Globals.level() == 0", known({ "Globals.level() == 0": true }))).toBe(true);
    expect(holds('Globals.has_item("coffee") and not Globals.has_item("paper")', known({ 'Globals.has_item("coffee")': true, 'Globals.has_item("paper")': false }))).toBe(true);
    expect(holds("x = = 2", known())).toEqual({ ask: "x = = 2" });
  });
});

describe("playing", () => {
  it("shows lines one at a time, and falls through into the next label", () => {
    const run = new Run(play([label("a", [say("Keeper", "One."), say(null, "Two.")]), label("b", [say("Keeper", "Three."), jump(end)])]), new PlayState());
    run.start("a");
    expect(lines(run)).toEqual(["Keeper: One."]);
    run.next();
    run.next();
    expect(lines(run)).toEqual(["Keeper: One.", "-: Two.", "Keeper: Three."]);
    run.next();
    expect(run.wait).toEqual({ kind: "end", why: "end" });
    expect(run.events.filter((e) => e.kind === "label").map((e) => e.kind === "label" && `${e.label}:${e.how}`)).toEqual(["a:start", "b:fall"]);
  });

  it("offers the choices whose conditions hold, and follows the one picked", () => {
    const state = new PlayState();
    state.flags.add("asked");
    const menu: Node = {
      ...base,
      type: "menu",
      choices: [choice("Ask again", [say("Keeper", "Again?")], 'has_flag("asked")'), choice("Never", [], 'not has_flag("asked")'), choice("Leave", [], null, to("bye"))],
    };
    const run = new Run(play([label("a", [say("Keeper", "Well?"), menu, say("Keeper", "After.")]), label("bye", [say("Keeper", "Bye."), jump(end)])]), state);
    run.start("a");
    run.next();
    expect(run.wait.kind === "choose" && run.wait.options.map((o) => o.text)).toEqual(["Ask again", "Leave"]);
    run.choose(0);
    run.next();
    expect(lines(run)).toEqual(["Keeper: Well?", "> Ask again", "Keeper: Again?", "Keeper: After."]);
  });

  it("asks what it cannot know, once, with a variant's names", () => {
    const branch: Node = {
      ...base,
      type: "branch",
      arms: [
        { keyword: "if", condition: "Globals.easy()", span, notes: [], marks: [], body: [say("Keeper", "Easy.")] },
        { keyword: "else", condition: null, span, notes: [], marks: [], body: [say("Keeper", "Hard.")] },
      ],
    };
    const state = new PlayState();
    const run = new Run(play([label("a", [branch, jump(to("a"))])]), state);
    run.start("a");
    expect(run.wait).toEqual({ kind: "ask", question: "Globals.easy()", yes: "Easy", no: "Hard" });
    run.answer(false);
    expect(lines(run)).toEqual(["Keeper: Hard."]);
    run.next();
    // Back at the start: the answer is remembered.
    expect(lines(run)).toEqual(["Keeper: Hard.", "Keeper: Hard."]);
  });

  it("sets flags and variables, and fills values into lines", () => {
    const effect = (keyword: string, expr: string): Node => ({ ...base, type: "effect", keyword, expr });
    const run = new Run(
      play([
        label("a", [
          effect("do", 'Globals.set_flag("met")'),
          effect("set", "gold = 2"),
          effect("set", "gold += 1"),
          effect("do", "Globals.shake()"),
          say("Keeper", "", { text: [{ type: "text", text: "You have " }, { type: "expr", expr: "gold" }, { type: "text", text: " coins." }] }),
        ]),
      ]),
      new PlayState(),
    );
    run.start("a");
    expect(lines(run)).toEqual(["Keeper: You have 3 coins."]);
    expect(run.state.flags.has("met")).toBe(true);
    expect(run.events.filter((e) => e.kind === "effect").map((e) => e.kind === "effect" && e.applied)).toEqual([true, true, true, false]);
  });

  it("comes back from a jump with return, and ends on END!", () => {
    const run = new Run(
      play([
        label("a", [say("Keeper", "Before."), jump(to("aside", true)), say("Keeper", "After."), { ...base, type: "jump", target: { kind: "end_conversation" }, returns: false }]),
        label("aside", [say("Gull", "Caw."), jump(end)]),
      ]),
      new PlayState(),
    );
    run.start("a");
    run.next();
    run.next();
    expect(lines(run)).toEqual(["Keeper: Before.", "Gull: Caw.", "Keeper: After."]);
    run.next();
    expect(run.wait).toEqual({ kind: "end", why: "end_conversation" });
  });

  it("says when a jump goes nowhere", () => {
    const run = new Run(play([label("a", [jump(to("nowhere"))])]), new PlayState());
    run.start("a");
    expect(run.wait).toEqual({ kind: "end", why: "missing" });
  });
});
