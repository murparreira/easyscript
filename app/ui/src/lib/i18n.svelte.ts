// The app's own words in the writer's language. A message is its English text, which is
// also its key: `t("Save")`, `t("{n} labels", { n })`. Other languages map the English to
// theirs (`locales/`); a message with no translation shows in English. A test checks that
// every message in the code has a translation, and that no translation is left over.

import ptBR from "./locales/pt-BR";

type Params = Record<string, string | number>;

export const LANGUAGES = [
  { id: "en", name: "English" },
  { id: "pt-BR", name: "Português (Brasil)" },
] as const;

export type Language = (typeof LANGUAGES)[number]["id"];

const CATALOGS: Record<Language, Record<string, string>> = {
  en: {},
  "pt-BR": ptBR,
};

const KEY = "dialogue-maker:language";

/** The language chosen before, or the system's when it is one there is. */
function initial(): Language {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved && saved in CATALOGS) return saved as Language;
  } catch {
    // No storage: follow the system.
  }
  const system = typeof navigator === "undefined" ? [] : navigator.languages ?? [navigator.language];
  for (const tag of system) {
    if (tag.toLowerCase().startsWith("pt")) return "pt-BR";
    if (tag.toLowerCase().startsWith("en")) return "en";
  }
  return "en";
}

class Locale {
  language: Language = $state(initial());

  set(language: Language) {
    this.language = language;
    try {
      localStorage.setItem(KEY, language);
    } catch {
      // Not remembered; it still applies now.
    }
    document.documentElement.lang = language;
  }
}

export const locale = new Locale();

function fill(text: string, params?: Params): string {
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in params ? String(params[name]) : whole));
}

/** A message in the writer's language, with `{name}` filled from `params`. */
export function t(message: string, params?: Params): string {
  return fill(CATALOGS[locale.language][message] ?? message, params);
}

/** `one` or `many` by `n` (both English messages with `{n}`), in the writer's language. */
export function plural(n: number, one: string, many: string, params?: Params): string {
  return t(n === 1 ? one : many, { n, ...params });
}
