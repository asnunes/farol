import { useHeldChoices } from "@/hooks/useHeldChoices";
import type { FileView } from "@/api";

/** Which files are open on the page.
 *
 * A file that has been read is closed: it is done, and the ones still to read
 * should not be buried under it. On a review the lenses speak for, every file
 * starts closed: the reader learns the change from them, and opens the code
 * when they want it. Anything the reader says overrides that — a
 * file they opened stays open however it is marked — but only for as long as
 * the thing it was overriding holds, which is `useHeldChoices`' rule, the one
 * the lenses fold by too. */
export function useOpenFiles() {
  const held = useHeldChoices();
  // Open unless it was read, or the lenses speak for it.
  const otherwise = (file: FileView) => !file.viewed && !file.folded;

  return {
    isOpen: (file: FileView) => held.isOpen(file.path, standing(file), otherwise(file)),
    set: (file: FileView, open: boolean) => held.set(file.path, standing(file), open),
    /** Hold the file as it is now through the read mark about to flip, for a
     * tick from the sidebar, which is not asking to see or to put away. */
    keep: (file: FileView) =>
      held.keep(file.path, standing(file), otherwise(file), standing({ ...file, viewed: !file.viewed })),
  };
}

export type OpenFiles = ReturnType<typeof useOpenFiles>;

/** What decides a file's opening on its own: whether it was read, and whether
 * the review is read through lenses. A choice the reader made lapses when
 * either changes. */
function standing(file: FileView): string {
  return `${file.viewed}:${file.folded}`;
}
