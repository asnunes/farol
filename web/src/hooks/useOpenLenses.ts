import { useState } from "react";

/** Which lenses of the pull request and of the blocks are open on the page.
 *
 * The same rule as `useOpenFiles`: a seen lens is folded, an unseen one open,
 * and a choice the reader made holds only while the mark it was made against
 * does. Held up here rather than in each section because the sidebar can tick
 * a lens without unfolding or folding it, and has to say so before the mark
 * changes under the section. */
export function useOpenLenses() {
  const [choice, setChoice] = useState<Record<string, Choice>>({});

  const isOpen = (key: string, seen: boolean) => {
    const said = choice[key];
    return said?.against === seen ? said.open : !seen;
  };

  return {
    isOpen,
    set: (key: string, seen: boolean, open: boolean) =>
      setChoice((all) => ({ ...all, [key]: { open, against: seen } })),
    /** Hold the lens as it is now through the mark about to change. */
    keep: (key: string, seen: boolean) =>
      setChoice((all) => ({ ...all, [key]: { open: isOpen(key, seen), against: !seen } })),
  };
}

export type OpenLenses = ReturnType<typeof useOpenLenses>;

/** What the reader asked for, and whether the lens was seen when they did. */
type Choice = { open: boolean; against: boolean };
