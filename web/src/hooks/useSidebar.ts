import { useCallback, useState } from "react";
import { useNarrow } from "@/hooks/useNarrow";

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
