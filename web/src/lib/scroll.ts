/** Bring a file to the top of the reading pane.
 *
 * On the next frame, not this one: the click that moves the reader usually
 * folds the file they were on, and scrolling before React has redrawn measures
 * the page as it was, landing halfway down the file it was aiming at. */
export function scrollToFile(path: string) {
  settling = Date.now() + SETTLES_IN;
  requestAnimationFrame(() => {
    document.querySelector(`[data-path="${CSS.escape(path)}"]`)?.scrollIntoView({ block: "start" });
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

let settling = 0;

const SETTLES_IN = 400;
