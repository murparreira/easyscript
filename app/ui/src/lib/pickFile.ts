// Pick a file in the project with the system's file chooser: its path in the project
// (relative, `/`-separated), or null when the writer cancels. Throws, with a sentence to
// show, for a file outside the project.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { projects } from "./app.svelte";
import { t } from "./i18n.svelte";

const IMAGES = ["png", "jpg", "jpeg", "webp", "gif"];

export async function pickImage(title: string, near?: string | null): Promise<string | null> {
  // Start where the images the project already names are, else at the project's root.
  const root = projects.current;
  const folder = near && root ? `${root}/${near.slice(0, near.lastIndexOf("/") + 1)}` : root;
  const chosen = await open({ title, multiple: false, directory: false, defaultPath: folder ?? undefined, filters: [{ name: t("Images"), extensions: IMAGES }] });
  if (typeof chosen !== "string") return null;
  return invoke<string>("project_path", { path: chosen });
}
