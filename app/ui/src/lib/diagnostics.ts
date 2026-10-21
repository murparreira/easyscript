// What a check found, in the writer's language: each code's message from its values
// (`args`, named in crates/core/src/check.rs). A code this does not know shows the
// English message the check wrote.

import type { Diagnostic } from "./bindings/Diagnostic";
import { plural, t } from "./i18n.svelte";

export function describe(d: Diagnostic): string {
  const a = d.args;
  const n = Number(a.count ?? 0);
  switch (d.code) {
    case "unreadable-file":
      return t("cannot read this file ({why}); it is left alone", { why: a.why ?? "" });
    case "syntax":
      return t("cannot read “{text}”", { text: a.text ?? "" });
    case "duplicate-label":
      return t("“{label}” is already a title in this file", { label: a.label ?? "" });
    case "unknown-jump-target":
      return a.label ? t("no title “{label}” to jump to", { label: a.label }) : t("jump with no target");
    case "unknown-speaker":
      return plural(n, "speaker “{speaker}” is no character's ({n} line)", "speaker “{speaker}” is no character's ({n} lines)", { speaker: a.speaker ?? "" });
    case "missing-portrait":
      return t("{character}'s portrait “{path}” does not exist", { character: a.character ?? "", path: a.path ?? "" });
    case "missing-voice":
      return plural(n, "{character} has no “{mood}” voice recorded ({n} line)", "{character} has no “{mood}” voice recorded ({n} lines)", {
        character: a.character ?? "",
        mood: a.mood ?? "",
      });
    case "bad-tag-value":
      switch (a.problem) {
        case "has-value":
          return t("“{tag}” takes no value, has “{value}”", { tag: a.tag ?? "", value: a.value ?? "" });
        case "needs-value":
          return t("“{tag}” needs a value", { tag: a.tag ?? "" });
        case "not-a-value":
          return t("“{value}” is not one of {tag}'s values ({values})", { tag: a.tag ?? "", value: a.value ?? "", values: a.values ?? "" });
      }
      return d.message;
    case "unknown-tag":
      return plural(n, "tag “{tag}” is not in the project file ({n} line)", "tag “{tag}” is not in the project file ({n} lines)", { tag: a.tag ?? "" });
    case "missing-asset":
      return plural(n, "“{path}” does not exist ({n} line)", "“{path}” does not exist ({n} lines)", { path: a.path ?? "" });
    case "unreachable-label":
      return t("nothing leads to “{label}”: no jump, no code naming it, not an entry point", { label: a.label ?? "" });
    case "unknown-place-file":
      return t("place “{place}”: “{path}” is not one of the dialogue files", { place: a.place ?? "", path: a.path ?? "" });
    case "unknown-place-label":
      return t("place “{place}”: no label “{label}” in its files", { place: a.place ?? "", label: a.label ?? "" });
    case "missing-document":
      return t("document “{path}” cannot be read", { path: a.path ?? "" });
    case "broken-link":
      return t("no label “{label}”", { label: a.label ?? "" });
  }
  return d.message;
}
