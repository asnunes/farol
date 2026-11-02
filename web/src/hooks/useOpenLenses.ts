import { useHeldChoices } from "@/hooks/useHeldChoices";

/** Which lenses of the pull request and of the blocks are open on the page.
 *
 * The same rule as `useOpenFiles`, through the same hook: a seen lens is
 * folded, an unseen one open, and a choice the reader made holds only while
 * the mark it was made against does. Held up here rather than in each section
 * because the sidebar can tick a lens without unfolding or folding it, and has
 * to say so before the mark changes under the section. */
export function useOpenLenses() {
  const held = useHeldChoices();

  return {
    isOpen: (key: string, seen: boolean) => held.isOpen(key, String(seen), !seen),
    set: (key: string, seen: boolean, open: boolean) => held.set(key, String(seen), open),
    /** Hold the lens as it is now through the mark about to change. */
    keep: (key: string, seen: boolean) => held.keep(key, String(seen), !seen, String(!seen)),
  };
}

export type OpenLenses = ReturnType<typeof useOpenLenses>;
