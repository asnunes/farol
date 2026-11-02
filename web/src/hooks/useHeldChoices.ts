import { useState } from "react";

/** Open or folded, as the reader chose, for as long as the thing the choice
 * was made against still holds.
 *
 * The one rule both the files and the lenses fold by. A choice used to be
 * kept forever, so a file folded away because it had been read stayed folded
 * after it stopped being read. Remembering the standing each choice was made
 * against lets it lapse on its own when that standing changes. */
export function useHeldChoices() {
  const [choice, setChoice] = useState<Record<string, Choice>>({});

  const isOpen = (key: string, standing: string, otherwise: boolean) => {
    const said = choice[key];
    return said?.against === standing ? said.open : otherwise;
  };

  return {
    isOpen,
    set: (key: string, standing: string, open: boolean) =>
      setChoice((all) => ({ ...all, [key]: { open, against: standing } })),
    /** Hold it as it is now through a change of standing about to happen, for
     * a tick that is not asking to see or to put away. */
    keep: (key: string, standing: string, otherwise: boolean, next: string) =>
      setChoice((all) => ({ ...all, [key]: { open: isOpen(key, standing, otherwise), against: next } })),
  };
}

/** What the reader asked for, and the standing it was asking against. */
type Choice = { open: boolean; against: string };
