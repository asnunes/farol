import { useCallback, useEffect, useState } from "react";

/** Which section of Settings the address names, or null on the review.
 *
 * The address and not a piece of state, so Back closes Settings, a reload
 * reopens the same section, and a section can be sent as a link. Three
 * sections and one screen do not earn a router: the history API is enough. */
export function useSettingsRoute(confirmLeave: () => boolean) {
  const [section, setSection] = useState<SettingsSection | null>(() => sectionOf(location.pathname));

  useEffect(() => {
    const onPop = () => {
      const next = sectionOf(location.pathname);
      // Back cannot be refused, so a refusal puts the address back.
      if (next === null && section !== null && !confirmLeave()) {
        history.pushState({ settings: true }, "", pathOf(section));
        return;
      }
      setSection(next);
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, [section, confirmLeave]);

  const open = useCallback(
    (next: SettingsSection) => {
      // Not guarded: each section keeps its drafts while Settings is open.
      if (next === section) return;
      // From the review, a step Back can undo; between sections, a swap, so
      // Back from any section returns to the review in one press.
      if (section === null) history.pushState({ settings: true }, "", pathOf(next));
      else history.replaceState(history.state, "", pathOf(next));
      setSection(next);
    },
    [section, confirmLeave],
  );

  const close = useCallback(() => {
    if (section === null || !confirmLeave()) return;
    // Opened from the review: Back is the way out, and leaves no entry behind.
    // Loaded straight onto Settings: there is nothing behind it to go back to.
    if (history.state?.settings) history.back();
    else history.replaceState(null, "", "/");
    setSection(null);
  }, [section, confirmLeave]);

  return { section, open, close };
}

export type SettingsSection = (typeof SECTIONS)[number];

export const SECTIONS = ["general", "instructions", "lenses"] as const;

function sectionOf(path: string): SettingsSection | null {
  const name = path.match(/^\/settings\/([a-z]+)\/?$/)?.[1];
  return SECTIONS.find((s) => s === name) ?? null;
}

function pathOf(section: SettingsSection | null): string {
  return section ? `/settings/${section}` : "/";
}
