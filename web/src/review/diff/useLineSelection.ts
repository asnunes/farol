import { useCallback, useEffect, useRef, useState } from "react";

/** Choosing the lines a comment is about, by dragging down the numbers.
 *
 * A comment is written about a passage, not a line, so picking the passage has
 * to be as cheap as reading it. Pressing on one number and releasing on another
 * covers the span whichever way it was dragged, and pressing and releasing on
 * the same number covers that one line — which is what the `+` in the gutter
 * does without the reader having to discover the drag. */
export function useLineSelection(): LineSelection {
  const dragging = useRef<Span | null>(null);
  const [drag, setDrag] = useState<Span | null>(null);
  const [composing, setComposing] = useState<Span | null>(null);

  // Bound once, and to the window rather than the gutter: the button comes up
  // wherever the pointer happens to be, including outside the diff, and a drag
  // that ended off the edge must not leave the page stuck selecting.
  useEffect(() => {
    const done = () => {
      const live = dragging.current;
      dragging.current = null;
      if (!live) return;
      setDrag(null);
      setComposing(ordered(live));
    };
    window.addEventListener("mouseup", done);
    return () => window.removeEventListener("mouseup", done);
  }, []);

  const start = useCallback((line: number) => {
    dragging.current = { from: line, to: line };
    setDrag(dragging.current);
  }, []);

  const extend = useCallback((line: number) => {
    if (!dragging.current) return;
    // A fresh object every time: mutating the one in the ref would leave React
    // with the same identity and nothing would redraw.
    dragging.current = { from: dragging.current.from, to: line };
    setDrag(dragging.current);
  }, []);

  const covers = useCallback(
    (line: number) => {
      if (!drag) return false;
      const span = ordered(drag);
      return span.from <= line && line <= span.to;
    },
    [drag],
  );

  return {
    composing,
    covers,
    start,
    extend,
    open: useCallback((line: number) => setComposing({ from: line, to: line }), []),
    close: useCallback(() => setComposing(null), []),
  };
}

export type Span = { from: number; to: number };

export type LineSelection = {
  /** The span the box is open on, if it is open. */
  composing: Span | null;
  /** Whether a line is inside the drag under way. */
  covers: (line: number) => boolean;
  start: (line: number) => void;
  extend: (line: number) => void;
  open: (line: number) => void;
  close: () => void;
};

/** Dragging upward is the same span as dragging downward. */
function ordered({ from, to }: Span): Span {
  return from <= to ? { from, to } : { from: to, to: from };
}
