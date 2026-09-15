import { useCallback, useEffect, useRef, useState } from "react";
import { numberOn } from "./line";
import type { Anchor } from "./line";
import type { DiffLine, Side } from "@/api";

/** Choosing the lines a comment is about: by dragging down the numbers, or
 * from the `+` those numbers carry.
 *
 * A comment is written about a passage, not a line, so picking the passage has
 * to be as cheap as reading it. Pressing on one number and releasing on another
 * covers the span whichever way it was dragged, and pressing and releasing on
 * the same number covers that one line — which is what the `+` in the gutter
 * does without the reader having to discover the drag.
 *
 * A drag is a mouse and nothing else, so the same choice is offered on the `+`
 * itself: `reach` walks the far end of the span a line at a time, the rows
 * light up the way they do under a drag, and the press that was already going
 * to open the box opens it over what was reached instead of over the one line.
 *
 * The span belongs to the side it started on, whichever way it was picked. A
 * drag that started in the old column and wandered into the new one stops where
 * the old side stops rather than jumping columns, and `reach` is handed the side
 * of the `+` it was pressed on: the two number separately, and a span that
 * changed sides halfway would cover lines nobody chose. */
export function useLineSelection(): LineSelection {
  const dragging = useRef(false);
  // The live span, and the same thing again as state. The ref is what the
  // handlers read — a drag and a `reach` both answer before React has redrawn
  // — and the state is what the rows are marked from.
  const held = useRef<Span | null>(null);
  const [picking, setPicking] = useState<Span | null>(null);
  const [composing, setComposing] = useState<Span | null>(null);

  const hold = useCallback((span: Span | null) => {
    held.current = span;
    setPicking(span);
  }, []);

  // Bound once, and to the window rather than the gutter: the button comes up
  // wherever the pointer happens to be, including outside the diff, and a drag
  // that ended off the edge must not leave the page stuck selecting.
  useEffect(() => {
    const done = () => {
      if (!dragging.current) return;
      dragging.current = false;
      const live = held.current;
      hold(null);
      if (live) setComposing(ordered(live));
    };
    window.addEventListener("mouseup", done);
    return () => window.removeEventListener("mouseup", done);
  }, [hold]);

  const start = useCallback(
    (side: Side, line: number) => {
      dragging.current = true;
      hold({ side, from: line, to: line });
    },
    [hold],
  );

  const extend = useCallback(
    (line: DiffLine) => {
      if (!dragging.current || !held.current) return;
      // The line as it is numbered on the side the drag started on, which a
      // line the other side alone has is not. Reached over, it is passed by
      // rather than converted into a number on a side nobody was pointing at.
      const at = numberOn(line, held.current.side);
      if (at === null) return;
      // A fresh object every time: mutating the one in the ref would leave
      // React with the same identity and nothing would redraw.
      hold({ side: held.current.side, from: held.current.from, to: at });
    },
    [hold],
  );

  const covers = useCallback(
    (at: Anchor) => {
      if (!picking) return false;
      const line = numberOn(at[picking.side], picking.side);
      if (line === null) return false;
      const span = ordered(picking);
      return span.from <= line && line <= span.to;
    },
    [picking],
  );

  return {
    composing,
    picking,
    covers,
    start,
    extend,
    reach: useCallback(
      (side: Side, from: number, to: number) => hold({ side, from, to }),
      [hold],
    ),
    drop: useCallback(() => {
      // A drag owns the span while it lasts; only the `+` gives one up.
      if (!dragging.current) hold(null);
    }, [hold]),
    open: useCallback(
      (side: Side, line: number) => {
        const live = held.current;
        hold(null);
        // The span reached from this very `+` is what opens. Anything else —
        // a drag that has already handed its span over, a `+` further down the
        // file — is one line, which is what pressing a `+` has always meant.
        setComposing(
          live && live.side === side && live.from === line
            ? ordered(live)
            : { side, from: line, to: line },
        );
      },
      [hold],
    ),
    close: useCallback(() => setComposing(null), []),
  };
}

export type Span = { side: Side; from: number; to: number };

export type LineSelection = {
  /** The span the box is open on, if it is open. */
  composing: Span | null;
  /** The span being chosen right now, by drag or from the keyboard, anchored
   * at the number the choosing started on. */
  picking: Span | null;
  /** Whether a row of the diff is inside the choice under way. */
  covers: (at: Anchor) => boolean;
  start: (side: Side, line: number) => void;
  extend: (line: DiffLine) => void;
  /** Put the far end of the span on a line, from the keyboard. */
  reach: (side: Side, from: number, to: number) => void;
  /** Give up a span that was reached but never opened. */
  drop: () => void;
  open: (side: Side, line: number) => void;
  close: () => void;
};

/** Dragging upward is the same span as dragging downward. */
function ordered({ side, from, to }: Span): Span {
  return from <= to ? { side, from, to } : { side, from: to, to: from };
}
