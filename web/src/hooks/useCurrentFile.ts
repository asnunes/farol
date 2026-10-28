import { useEffect, useRef } from "react";
import { isSettling } from "@/lib/scroll";
import type { ReviewView } from "@/api";

/** Name the file the reader is on, from where the pane is scrolled.
 *
 * The file whose header last passed a line a quarter down the pane, which is
 * the one being worked on, and the first one on screen before any has.
 *
 * The pane keeps most of a screen of room under the last file, so every file
 * can be brought up to the line. Without that room the tail of the review
 * cannot reach it, and picking the last visible file instead makes the whole
 * last screenful report the same one: every file but the bottom one becomes
 * impossible to mark from the keyboard.
 *
 * Measured on scroll rather than watched with an observer: an observer reports
 * crossings, and neither end of the list produces one. */
export function useCurrentFile(
  pane: React.RefObject<HTMLElement | null>,
  review: ReviewView,
  onCurrent: (path: string) => void,
) {
  const onCurrentRef = useRef(onCurrent);
  onCurrentRef.current = onCurrent;

  useEffect(() => {
    const root = pane.current;
    if (!root) return;

    let queued = 0;
    const look = () => {
      if (isSettling()) return;
      const found = whereTheReaderIs(root);
      if (found) onCurrentRef.current(found);
    };
    const onScroll = () => {
      cancelAnimationFrame(queued);
      queued = requestAnimationFrame(look);
    };

    look();
    root.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      cancelAnimationFrame(queued);
      root.removeEventListener("scroll", onScroll);
    };
  }, [pane, review]);
}

function whereTheReaderIs(root: HTMLElement): string | null {
  const pane = root.getBoundingClientRect();
  const line = pane.top + pane.height * READING_LINE;

  // The pull request's and the blocks' lenses are places too, so the sidebar
  // and the mark key follow the reader onto them. A file's own lenses are not:
  // they sit under its header and belong to it.
  const visible = [...root.querySelectorAll<HTMLElement>(PLACES)]
    .map((el) => ({ el, box: el.getBoundingClientRect() }))
    .filter(({ box }) => box.bottom > pane.top && box.top < pane.bottom);
  if (!visible.length) return null;

  const passed = visible.filter(({ box }) => box.top <= line);
  const here = passed[passed.length - 1] ?? visible[0];

  return placeOf(here.el);
}

/** The place after this one on the page: the next lens or file, in the order
 * they are drawn. What marking moves on to, so a press that folds a lens away
 * does not leave the reader on it for the next press to unfold. */
export function placeAfter(current: string | null): string | null {
  const places = [...document.querySelectorAll<HTMLElement>(PLACES)];
  const at = places.findIndex((el) => placeOf(el) === current);
  return at < 0 ? null : (places.slice(at + 1).map(placeOf).find(Boolean) ?? null);
}

function placeOf(el: HTMLElement): string | null {
  const { path, lensKey } = el.dataset;
  return path ?? (lensKey ? `lens:${lensKey}` : null);
}

/** How far down the pane a header has to be before the file counts as the one
 * being read. Near the top, because the reader works downwards. */
const READING_LINE = 0.25;

const PLACES = '[data-path], [data-lens-key^="pr:"], [data-lens-key^="block:"]';
