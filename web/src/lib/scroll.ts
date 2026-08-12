/** Bring a file to the top of the reading pane. */
export function scrollToFile(path: string) {
  document.querySelector(`[data-path="${CSS.escape(path)}"]`)?.scrollIntoView({ block: "start" });
}
