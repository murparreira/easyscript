// Opening projects, from anywhere in the app (the welcome screen, settings, a clone from
// GitHub). The app shell sets how.

export const projects: { open: (path: string) => void; current: string | null } = $state({
  open: () => {},
  /** The folder of the project open, in the desktop app. */
  current: null,
});
