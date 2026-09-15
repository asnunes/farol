import { useCallback, useState, useSyncExternalStore } from "react";

/** Whether the sidebar is showing, and the two ways it stops.
 *
 * It starts on where there is width to hold it beside the code, and off where
 * there is not: below `md` it has nowhere to go but above the diff, and a
 * review that opens on its own table of contents is a review that opens on
 * nothing to read. Not remembered between visits either way — the screen
 * decides, and ⌘B overrules it for as long as the reader is here. */
export function useSidebar(): Sidebar {
  const narrow = useNarrow();
  const [open, setOpen] = useState(!narrow);

  // Crossing the width decides the sidebar again rather than carrying the last
  // answer over, so dragging a window narrow does not leave the column sitting
  // on top of the code. Written during render, which is React's own answer to
  // state that follows something else: an effect would draw the wrong layout
  // first and correct it on the next frame.
  const [decidedFor, setDecidedFor] = useState(narrow);
  if (narrow !== decidedFor) {
    setDecidedFor(narrow);
    setOpen(!narrow);
  }

  return {
    open,
    toggle: useCallback(() => setOpen((showing) => !showing), []),
    dismiss: useCallback(() => {
      // Only where it is in the way. Beside the code, having picked a file is
      // no reason to lose the list it was picked from.
      if (narrow) setOpen(false);
      return narrow;
    }, [narrow]),
  };
}

export type Sidebar = {
  open: boolean;
  toggle: () => void;
  /** Put it away if it is over the code rather than beside it, and say whether
   * that happened — what was focused inside it has gone with it. */
  dismiss: () => boolean;
};

/** Too narrow to give the sidebar a column of its own.
 *
 * The query is what Tailwind's `max-md:` compiles to, spelled out here so the
 * one line that decides this in JavaScript and the classes that lay it out in
 * CSS cannot drift apart. */
const NARROW = "not all and (min-width: 48rem)";

function useNarrow(): boolean {
  return useSyncExternalStore(watch, () => query()?.matches ?? false, () => false);
}

/** jsdom has no `matchMedia`, and a test that is not about the width should not
 * have to stand one up. Missing, it reads as the wide screen the app was
 * written for. */
function query(): MediaQueryList | null {
  return typeof window !== "undefined" && window.matchMedia
    ? window.matchMedia(NARROW)
    : null;
}

function watch(onChange: () => void): () => void {
  const media = query();
  media?.addEventListener("change", onChange);
  return () => media?.removeEventListener("change", onChange);
}
