/** Bring a file to the top of the reading pane.
 *
 * On the next frame, not this one: the click that moves the reader usually
 * folds the file they were on, and scrolling before React has redrawn measures
 * the page as it was, landing halfway down the file it was aiming at. */
export function scrollToFile(path: string) {
  settling = Date.now() + SETTLES_IN;
  requestAnimationFrame(() => {
    const file = document.querySelector<HTMLElement>(`[data-path="${CSS.escape(path)}"]`);
    file?.scrollIntoView({ block: "start" });

    // A file that opens a block is reached through the block. The band above it
    // carries the title and the paragraph saying what the next few files are
    // for, and it was written to be read before them — landing on the file
    // scrolls straight past it, and the reader only meets it by going back up.
    opens(file)?.scrollIntoView({ block: "start" });
  });
}

/** Whether a move is still landing.
 *
 * Folding a file shortens the page under the scroll that is already on its way,
 * so for a moment the position says one thing and the reader was sent to
 * another. Whoever asked for the move wins until it settles. */
export function isSettling() {
  return Date.now() < settling;
}

/** The block band this file opens, if it opens one.
 *
 * The band is the section's own previous sibling: the pane lays blocks and
 * files out as one flat list, in reading order, so only the first file of a
 * block has one before it. */
function opens(file: HTMLElement | null | undefined) {
  const before = file?.previousElementSibling;
  return before?.classList.contains("blockbar") ? (before as HTMLElement) : null;
}

let settling = 0;

const SETTLES_IN = 400;
