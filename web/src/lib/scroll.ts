/** Bring a file to the top of the reading pane.
 *
 * On the next frame, not this one: the click that moves the reader usually
 * folds the file they were on, and scrolling before React has redrawn measures
 * the page as it was, landing halfway down the file it was aiming at. */
export function scrollToFile(path: string) {
  stopAnchoring?.();
  settling = Date.now() + SETTLES_IN;
  requestAnimationFrame(() => {
    const file = document.querySelector<HTMLElement>(
      `[data-path="${CSS.escape(path)}"]`,
    );
    file?.scrollIntoView({ block: "start" });

    // A file that opens a block is reached through the block. The band above it
    // carries the title and the paragraph saying what the next few files are
    // for, and it was written to be read before them — landing on the file
    // scrolls straight past it, and the reader only meets it by going back up.
    const anchor = opens(file) ?? file;
    anchor?.scrollIntoView({ block: "start" });
    if (anchor) holdPosition(anchor);
  });
}

/** Bring a thread to the middle of the reading pane, and mark it for a moment.
 *
 * Called after the move to its file, which is what gets the file's diff asked
 * for at all: a diff is fetched when its file comes near the screen, and the
 * thread is drawn from it. So this waits for the thread, for as long as a diff
 * takes to arrive and no longer — a thread that never shows was closed or left
 * the diff, and there is nothing to land on. */
export function scrollToThread(id: string) {
  const giveUp = Date.now() + WAITS_FOR_THREAD;
  const look = () => {
    const thread = document.querySelector<HTMLElement>(`[data-thread="${CSS.escape(id)}"]`);
    if (!thread) {
      if (Date.now() < giveUp) requestAnimationFrame(look);
      return;
    }
    // The move to the file is holding the file in place; this move wins.
    stopAnchoring?.();
    settling = Date.now() + SETTLES_IN;
    thread.scrollIntoView({ block: "center" });
    holdPosition(thread);
    thread.classList.add("thread-flash");
    window.setTimeout(() => thread.classList.remove("thread-flash"), FLASHES_FOR);
  };
  requestAnimationFrame(look);
}

/** Bring a line of a file to the middle of the pane, and mark it for a moment.
 *
 * Waits for the row the way a thread jump does: the file was just opened, and
 * its diff arrives when it comes near. A line the diff does not print is never
 * drawn, and the jump stays on the file. */
export function scrollToLine(path: string, side: "old" | "new", line: number) {
  const giveUp = Date.now() + WAITS_FOR_THREAD;
  let approached = false;
  const look = () => {
    const file = document.querySelector<HTMLElement>(`[data-path="${CSS.escape(path)}"]`);
    const row = file?.querySelector<HTMLElement>(`.row[data-${side}="${line}"]`);
    if (!row) {
      // A long diff only draws the windows near the screen. Bring the one
      // holding the line near once, and it draws the row this is waiting for.
      const window = !approached && file ? windowHolding(file, side, line) : null;
      if (window) {
        approached = true;
        stopAnchoring?.();
        settling = Date.now() + SETTLES_IN;
        window.scrollIntoView({ block: "center" });
      }
      if (Date.now() < giveUp) requestAnimationFrame(look);
      return;
    }
    stopAnchoring?.();
    settling = Date.now() + SETTLES_IN;
    row.scrollIntoView({ block: "center" });
    holdPosition(row);
    row.classList.add("thread-flash");
    window.setTimeout(() => row.classList.remove("thread-flash"), FLASHES_FOR);
  };
  requestAnimationFrame(look);
}

/** The window of a file's diff that holds a line, drawn or not. */
function windowHolding(file: HTMLElement, side: "old" | "new", line: number) {
  for (const window of file.querySelectorAll<HTMLElement>(".diff-window")) {
    const from = Number(window.dataset[`${side}From`]);
    const to = Number(window.dataset[`${side}To`]);
    if (from <= line && line <= to) return window;
  }
  return null;
}

/** Nearby lazy diffs can grow above a jump after its first scroll lands. */
function holdPosition(anchor: HTMLElement) {
  const pane = anchor.closest<HTMLElement>(".pane");
  if (!pane || typeof ResizeObserver === "undefined") return;
  const top = anchor.getBoundingClientRect().top;
  const observer = new ResizeObserver(() => {
    if (!anchor.isConnected) {
      release();
      return;
    }
    const movement = anchor.getBoundingClientRect().top - top;
    if (Math.abs(movement) > 1) pane.scrollTop += movement;
  });
  // The pane's viewport stays the same size when its children grow.
  for (const child of pane.children) observer.observe(child);
  const timer = window.setTimeout(release, SETTLES_IN);
  const events = ["wheel", "touchstart", "pointerdown", "keydown"] as const;
  for (const event of events)
    window.addEventListener(event, release, { passive: true });
  function release() {
    observer.disconnect();
    window.clearTimeout(timer);
    for (const event of events) window.removeEventListener(event, release);
    if (stopAnchoring === release) stopAnchoring = undefined;
  }
  stopAnchoring = release;
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
  // The block's lenses and the line where its code starts sit between the
  // band and its first file, and belong to the band.
  let before = file?.previousElementSibling;
  while (before && (before.classList.contains("lens") || before.classList.contains("codebar"))) {
    before = before.previousElementSibling;
  }
  return before?.classList.contains("blockbar")
    ? (before as HTMLElement)
    : null;
}

let settling = 0;
let stopAnchoring: (() => void) | undefined;

const SETTLES_IN = 400;

/** How long a jump waits for its thread to be drawn: about what a diff takes. */
const WAITS_FOR_THREAD = 3000;

/** How long a thread reached from the activity box stays marked. */
const FLASHES_FOR = 1600;
