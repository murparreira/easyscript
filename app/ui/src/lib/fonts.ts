// The project's own fonts (`[look]` in its project file), for titles and for what is said.
// Loaded from the bytes in the view, so no font is fetched from anywhere.

import type { Fonts } from "./bindings/Fonts";

const loaded = new Map<string, string>();

/** Use the project's fonts, or the app's own when it has none. */
export async function useFonts(fonts: Fonts | undefined) {
  const parts: [keyof Fonts, string][] = [
    ["title", "--project-title"],
    ["speech", "--project-speech"],
  ];
  for (const [kind, variable] of parts) {
    const uri = fonts?.[kind] ?? null;
    const style = document.documentElement.style;
    if (!uri) {
      style.removeProperty(variable);
      loaded.delete(kind);
      continue;
    }
    if (loaded.get(kind) === uri) continue;
    try {
      const bytes = Uint8Array.from(atob(uri.slice(uri.indexOf(",") + 1)), (c) => c.charCodeAt(0));
      // A new family name each time, so a changed font is not the old one cached.
      const family = `dm-${kind}-${loaded.size}-${uri.length}`;
      const face = new FontFace(family, bytes.buffer);
      await face.load();
      document.fonts.add(face);
      style.setProperty(variable, `"${family}"`);
      loaded.set(kind, uri);
    } catch {
      // Not a font the browser reads: the app's own stays.
      style.removeProperty(variable);
    }
  }
}
