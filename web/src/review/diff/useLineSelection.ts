import { useCallback, useEffect, useRef, useState } from "react";
import { numberOn } from "./line";
import type { Anchor } from "./line";
import type { DiffLine, Side } from "@/api";

/** Choosing the lines a comment is about, by dragging down the numbers.
 *
 * A comment is written about a passage, not a line, so picking the passage has
 * to be as cheap as reading it. Pressing on one number and releasing on another
 * covers the span whichever way it was dragged, and pressing and releasing on
 * the same number covers that one line — which is what the `+` in the gutter
 * does without the reader having to discover the drag.
 *
 * The span belongs to the side it started on. A drag that started in the old
 * column and wandered into the new one stops where the old side stops rather
 * than jumping columns: the two number separately, and a span that changed
 * sides halfway would cover lines nobody dragged over. */
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

  const start = useCallback((side: Side, line: number) => {
    dragging.current = { side, from: line, to: line };
    setDrag(dragging.current);
  }, []);

  const extend = useCallback((line: DiffLine) => {
    if (!dragging.current) return;
    // The line as it is numbered on the side the drag started on, which a line
    // the other side alone has is not. Reached over, it is passed by rather
    // than converted into a number on a side nobody was pointing at.
    const at = numberOn(line, dragging.current.side);
    if (at === null) return;
    // A fresh object every time: mutating the one in the ref would leave React
    // with the same identity and nothing would redraw.
    dragging.current = { side: dragging.current.side, from: dragging.current.from, to: at };
    setDrag(dragging.current);
  }, []);

  const covers = useCallback(
    (at: Anchor) => {
      if (!drag) return false;
      const line = numberOn(at[drag.side], drag.side);
      if (line === null) return false;
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
    open: useCallback(
      (side: Side, line: number) => setComposing({ side, from: line, to: line }),
      [],
    ),
    close: useCallback(() => setComposing(null), []),
  };
}

export type Span = { side: Side; from: number; to: number };

export type LineSelection = {
  /** The span the box is open on, if it is open. */
  composing: Span | null;
  /** Whether a row of the diff is inside the drag under way. */
  covers: (at: Anchor) => boolean;
  start: (side: Side, line: number) => void;
  extend: (line: DiffLine) => void;
  open: (side: Side, line: number) => void;
  close: () => void;
};

/** Dragging upward is the same span as dragging downward. */
function ordered({ side, from, to }: Span): Span {
  return from <= to ? { side, from, to } : { side, from: to, to: from };
}
