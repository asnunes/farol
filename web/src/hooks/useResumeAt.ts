import { useEffect, useRef } from "react";
import { scrollToFile } from "@/lib/scroll";

/** Take the reader to where they stopped, once, when the review arrives.
 *
 * The page opens at the top otherwise, and the first unread file is halfway
 * down a review that has been read before.
 *
 * Landing takes more than one scroll. The diffs are fetched as the reader
 * nears them, so the page is still growing when the first one happens — on a
 * review of sixty files the content doubles in height within a frame or two of
 * the landing, and every file that grows above the target drags it out of
 * sight. The scroll offset stays where it was put, no scroll event fires, and
 * nothing recomputes which file is being read: the sidebar goes on pointing at
 * the file the reader was taken to while the pane shows whichever one happened
 * to land at that offset. So the aim is corrected until the page stops moving
 * underneath it. */
export function useResumeAt(path: string | null) {
  const landed = useRef(false);
  // Outside the effect: the current file changes as soon as the landing works,
  // which re-runs the effect, and a flag inside it would have the aim give up
  // on the very move it just made.
  const taken = useRef(false);

  useEffect(() => {
    if (landed.current || !path) return;
    landed.current = true;

    // Straight away, not on the next frame: this is what claims the settling
    // window, and until it is claimed the scroll-derived current file is free
    // to name whatever happens to be at the top of an unscrolled pane.
    scrollToFile(path);

    let frames = 0;
    let wasAt = document.querySelector<HTMLElement>(`[data-path="${CSS.escape(path)}"]`)?.offsetTop ?? -1;

    const aim = () => {
      if (taken.current || frames++ > HOLD_FOR) return;

      const target = document.querySelector<HTMLElement>(`[data-path="${CSS.escape(path)}"]`);
      if (target) {
        // Where the file sits in the page, not where it sits on the screen: the
        // screen position is what is being corrected, so it cannot be what is
        // measured. It moves whenever anything above it grows or folds away.
        const at = target.offsetTop;
        if (at !== wasAt) {
          wasAt = at;
          scrollToFile(path);
        }
      }
      requestAnimationFrame(aim);
    };

    // The reader wins the moment they ask for anything. Landing is a courtesy,
    // and a courtesy that fights the wheel is a bug.
    const handOver = () => {
      taken.current = true;
    };
    const listening = { passive: true, once: true } as const;
    window.addEventListener("wheel", handOver, listening);
    window.addEventListener("touchstart", handOver, listening);
    window.addEventListener("keydown", handOver, { once: true });

    requestAnimationFrame(aim);
    return () => {
      window.removeEventListener("wheel", handOver);
      window.removeEventListener("touchstart", handOver);
      window.removeEventListener("keydown", handOver);
    };
  }, [path]);
}

/** How long the aim is held, in frames — about two seconds.
 *
 * Not "until it stops moving": the page goes still between the diffs arriving
 * and the read files folding away, and a landing that took the first stillness
 * for the last one stopped exactly one step early. It watches the whole window
 * instead, and only moves when the target actually moved. The reader takes it
 * back the moment they touch anything, so the ceiling is a backstop rather than
 * a promise. */
const HOLD_FOR = 120;
