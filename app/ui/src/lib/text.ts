// Text helpers for search: matching that ignores case and accents, mapped back to the
// original text so matches can be highlighted where they are.

/** Lowercase, accents removed: "Avó" and "avo" match. */
export function normalize(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Where `query` (already normalized) occurs in `text`, as [start, end) in `text`. */
export function matches(text: string, query: string): Array<[number, number]> {
  if (!query) return [];
  // Normalize character by character, remembering where each normalized char came from.
  let folded = "";
  const from: number[] = [];
  let i = 0;
  for (const char of text) {
    const n = normalize(char);
    for (let k = 0; k < n.length; k++) from.push(i);
    folded += n;
    i += char.length;
  }
  from.push(text.length);
  const out: Array<[number, number]> = [];
  let at = folded.indexOf(query);
  while (at >= 0) {
    const last = from[at + query.length - 1];
    const end = last + (text.codePointAt(last)! > 0xffff ? 2 : 1);
    out.push([from[at], end]);
    at = folded.indexOf(query, at + query.length);
  }
  return out;
}

export type Piece = { text: string; hit: boolean };

/** `text` cut into pieces that match `query` and pieces that do not. */
export function pieces(text: string, query: string): Piece[] {
  const hits = matches(text, query);
  if (hits.length === 0) return [{ text, hit: false }];
  const out: Piece[] = [];
  let pos = 0;
  for (const [start, end] of hits) {
    if (start > pos) out.push({ text: text.slice(pos, start), hit: false });
    out.push({ text: text.slice(start, end), hit: true });
    pos = end;
  }
  if (pos < text.length) out.push({ text: text.slice(pos), hit: false });
  return out;
}

/** Text split into stage directions "(like this)" and the rest. */
export function parentheticals(text: string): Array<{ text: string; paren: boolean }> {
  const out: Array<{ text: string; paren: boolean }> = [];
  const re = /\([^()]*\)/g;
  let pos = 0;
  for (const m of text.matchAll(re)) {
    if (m.index! > pos) out.push({ text: text.slice(pos, m.index), paren: false });
    out.push({ text: m[0], paren: true });
    pos = m.index! + m[0].length;
  }
  if (pos < text.length) out.push({ text: text.slice(pos), paren: false });
  return out;
}

/** Up to two initials, for a speaker without a face. */
export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0])
    .join("")
    .toUpperCase();
}
