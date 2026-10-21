// Every message in the code has a translation, with the same `{placeholders}`, and no
// translation is left over from a message that is gone.
import { describe, expect, test } from "vitest";
import ptBR from "./locales/pt-BR";

// The app's sources, as text.
const SOURCES = import.meta.glob(["../App.svelte", "./**/*.svelte", "./**/*.ts", "!./**/*.test.ts", "!./bindings/**", "!./locales/**"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const STRING = String.raw`"((?:[^"\\]|\\.)*)"`;
const CALLS = [
  new RegExp(String.raw`\bt\(\s*${STRING}`, "g"),
  new RegExp(String.raw`\bplural\([^,()]*(?:\([^()]*\))?[^,()]*,\s*${STRING},\s*${STRING}`, "g"),
];

function messages(): Map<string, string> {
  const found = new Map<string, string>();
  for (const [file, text] of Object.entries(SOURCES)) {
    for (const call of CALLS) {
      for (const m of text.matchAll(call)) {
        for (const raw of m.slice(1)) found.set(JSON.parse(`"${raw}"`), file);
      }
    }
  }
  return found;
}

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("pt-BR", () => {
  const used = messages();

  test("the code has messages", () => {
    expect(used.size).toBeGreaterThan(100);
  });

  test("every message is translated", () => {
    const missing = [...used].filter(([m]) => !(m in ptBR)).map(([m, file]) => `${file}: ${m}`);
    expect(missing).toEqual([]);
  });

  test("no translation is left over", () => {
    expect(Object.keys(ptBR).filter((m) => !used.has(m))).toEqual([]);
  });

  test("translations keep the placeholders", () => {
    const wrong = Object.entries(ptBR).filter(([m, tr]) => placeholders(m).join() !== placeholders(tr).join());
    expect(wrong).toEqual([]);
  });
});
