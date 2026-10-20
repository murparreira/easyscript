import { describe, expect, it } from "vitest";
import type { Arm } from "./bindings/Arm";
import type { VariantView } from "./bindings/VariantView";
import { armSides, conditionSide, exprPieces } from "./model";

const mode: VariantView = { name: "Modo", condition: "Globals.modo_assistido()", if_true: "Assistido", if_false: "Detetive" };
const span = { file: 0, start: 0, end: 0 };
const arm = (keyword: Arm["keyword"], condition: string | null): Arm => ({ keyword, condition, span, notes: [], marks: [], body: [] });

describe("variants", () => {
  it("reads a condition and its negation", () => {
    expect(conditionSide("Globals.modo_assistido()", [mode])?.side).toBe(true);
    expect(conditionSide("not Globals.modo_assistido()", [mode])?.side).toBe(false);
    expect(conditionSide("!Globals.modo_assistido()", [mode])?.side).toBe(false);
    expect(conditionSide("Globals.other()", [mode])).toBeNull();
  });

  it("puts an else on the other side of its if", () => {
    const sides = armSides([arm("if", "Globals.modo_assistido()"), arm("else", null)], [mode]);
    expect(sides.map((s) => s?.side)).toEqual([true, false]);
    // With an elif in between, the else is not simply the other side.
    const three = armSides([arm("if", "Globals.modo_assistido()"), arm("elif", "x"), arm("else", null)], [mode]);
    expect(three.map((s) => s?.side ?? null)).toEqual([true, null, null]);
  });
});

describe("flags in expressions", () => {
  const functions = { set: ["set_game_flag"], read: ["has_game_flag"], match_subjects: [] };

  it("picks out flag calls", () => {
    expect(exprPieces('not Globals.has_game_flag("asked") and x', functions)).toEqual([
      { text: "not " },
      { flag: "asked", access: "read" },
      { text: " and x" },
    ]);
    expect(exprPieces('set_game_flag(&"done")', functions)).toEqual([{ flag: "done", access: "set" }]);
  });

  it("leaves expressions alone without flag functions", () => {
    expect(exprPieces("a == 1", { set: [], read: [], match_subjects: [] })).toEqual([{ text: "a == 1" }]);
  });
});
