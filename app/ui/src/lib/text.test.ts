import { describe, expect, it } from "vitest";
import { initials, matches, normalize, parentheticals, pieces } from "./text";

describe("search text", () => {
  it("ignores case and accents", () => {
    expect(normalize("Avó Ovelha")).toBe("avo ovelha");
    expect(matches("Pé de cabra, PE DE CABRA", "pe de cabra")).toEqual([
      [0, 11],
      [13, 24],
    ]);
  });

  it("maps matches back through decomposed and astral characters", () => {
    const text = "áb 🙂 café";
    const [[start, end]] = matches(text, "cafe");
    expect(text.slice(start, end)).toBe("café");
    expect(matches("ab", "")).toEqual([]);
  });

  it("cuts text into hits and the rest", () => {
    expect(pieces("Tá tudo nesse jornal", "jornal")).toEqual([
      { text: "Tá tudo nesse ", hit: false },
      { text: "jornal", hit: true },
    ]);
  });

  it("finds stage directions", () => {
    expect(parentheticals("(Bufando) Buf!")).toEqual([
      { text: "(Bufando)", paren: true },
      { text: " Buf!", paren: false },
    ]);
    expect(initials("Gato Advogado da Silva")).toBe("GA");
  });
});
