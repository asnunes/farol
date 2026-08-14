/** The one channel the server pushes news down.
 *
 * Shared rather than opened per hook: the map and the comments both want to
 * hear about changes, and two `EventSource` objects on the same URL would be
 * two connections held open for one stream. */
export function onNudge(kind: Nudge, fn: () => void) {
  const source = open();
  source.addEventListener(kind, fn);

  return () => {
    source.removeEventListener(kind, fn);
    listeners--;
    // The last one out closes it, so a page that unmounts everything does not
    // leave a connection hanging.
    if (listeners === 0) {
      source.close();
      channel = null;
    }
  };
}

/** What the server has to say. `map` and `head` mean the review underneath the
 * reader moved; `comments` means only what the reader wrote did. */
export type Nudge = "map" | "head" | "comments";

function open(): EventSource {
  listeners++;
  channel ??= new EventSource("/api/watch");
  return channel;
}

let channel: EventSource | null = null;
let listeners = 0;
