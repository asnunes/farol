/** Bring a file to the top of the reading pane.
 *
 * On the next frame, not this one: the click that moves the reader usually
 * folds the file they were on, and scrolling before React has redrawn measures
 * the page as it was, landing halfway down the file it was aiming at. */
export function scrollToFile(path: string) {
  requestAnimationFrame(() => {
    document.querySelector(`[data-path="${CSS.escape(path)}"]`)?.scrollIntoView({ block: "start" });
  });
}
