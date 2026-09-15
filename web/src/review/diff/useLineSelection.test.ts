import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { sidesOf } from "./line";
import { useLineSelection } from "./useLineSelection";
import type { DiffLine } from "@/api";

/** Let go of the mouse, wherever the pointer is. */
function release() {
  act(() => {
    window.dispatchEvent(new MouseEvent("mouseup"));
  });
}

/** A line that is on both sides, under a different number on each — which is
 * every context line, and the case the side has to be carried for. */
function context(old: number, now: number): DiffLine {
  return { kind: "context", oldNumber: old, newNumber: now, content: "" };
}

function removed(old: number): DiffLine {
  return { kind: "removed", oldNumber: old, newNumber: null, content: "" };
}

function added(now: number): DiffLine {
  return { kind: "added", oldNumber: null, newNumber: now, content: "" };
}

describe("picking the lines a comment is about", () => {
  it("covers the span that was dragged", () => {
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("new", 3));
    act(() => result.current.extend(added(5)));
    release();

    expect(result.current.composing).toEqual({ side: "new", from: 3, to: 5 });
  });

  it("reads a drag upward as the same span as a drag downward", () => {
    // The reader drags from the line that puzzled them, which is as often the
    // last line of the passage as the first.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("new", 5));
    act(() => result.current.extend(added(3)));
    release();

    expect(result.current.composing).toEqual({ side: "new", from: 3, to: 5 });
  });

  it("takes a press and a release on one number as that one line", () => {
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("new", 7));
    release();

    expect(result.current.composing).toEqual({ side: "new", from: 7, to: 7 });
  });

  it("counts a drag on the old side on the old numbers", () => {
    // Dragging across removed lines, including a file that lost all of them.
    // The span that comes out is the one GitHub will be told about, on LEFT.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("old", 40));
    act(() => result.current.extend(removed(43)));
    release();

    expect(result.current.composing).toEqual({ side: "old", from: 40, to: 43 });
  });

  it("follows a drag on the old side through the context around it", () => {
    // A context line is on both sides under two different numbers. Reached
    // during a drag on the old side, it extends the span by its old number —
    // the new one would jump the selection somewhere nobody dragged.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("old", 40));
    act(() => result.current.extend(context(44, 60)));
    release();

    expect(result.current.composing).toEqual({ side: "old", from: 40, to: 44 });
  });

  it("stops at the edge of its own side rather than crossing to the other", () => {
    // The gesture that crosses from removals into additions. Half of it is on
    // a numbering the span is not counted in, so it is passed over: a span
    // that changed sides halfway would cover lines nobody dragged across.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("old", 40));
    act(() => result.current.extend(removed(41)));
    act(() => result.current.extend(added(41)));
    release();

    expect(result.current.composing).toEqual({ side: "old", from: 40, to: 41 });
  });

  it("marks the lines under the pointer while the drag is under way", () => {
    // Without this the reader is choosing a passage blind and only finds out
    // what they picked once the box is open.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("new", 3));
    act(() => result.current.extend(added(5)));

    expect([2, 3, 4, 5, 6].map((n) => result.current.covers(sidesOf(added(n))))).toEqual([
      false,
      true,
      true,
      true,
      false,
    ]);
  });

  it("marks only the side being dragged on", () => {
    // Line 4 exists on both sides of the same row. The drag is on the old one,
    // so the row is marked for the line it removed and not for the one that
    // replaced it.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start("old", 3));
    act(() => result.current.extend(removed(4)));

    expect(result.current.covers({ old: removed(4), new: null })).toBe(true);
    expect(result.current.covers({ old: null, new: added(4) })).toBe(false);
  });

  it("stops marking once the box is open", () => {
    const { result } = renderHook(() => useLineSelection());
    act(() => result.current.start("new", 3));
    act(() => result.current.extend(added(5)));

    release();

    expect(result.current.covers(sidesOf(added(4)))).toBe(false);
  });

  it("ignores a release that no press started", () => {
    // The mouse comes up all over the page for reasons that have nothing to do
    // with the gutter.
    const { result } = renderHook(() => useLineSelection());

    release();

    expect(result.current.composing).toBeNull();
  });

  it("closes without leaving the span behind", () => {
    const { result } = renderHook(() => useLineSelection());
    act(() => result.current.open("new", 9));

    act(() => result.current.close());

    expect(result.current.composing).toBeNull();
  });
});

describe("picking the lines from the plus, without a mouse", () => {
  it("opens the box over everything reached, not over the line pressed", () => {
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.reach("new", 3, 5));
    act(() => result.current.open("new", 3));

    expect(result.current.composing).toEqual({ side: "new", from: 3, to: 5 });
  });

  it("reads a reach upward as the same span as a reach downward", () => {
    // Shift and the up arrow, from the line that puzzled the reader.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.reach("new", 5, 3));
    act(() => result.current.open("new", 5));

    expect(result.current.composing).toEqual({ side: "new", from: 3, to: 5 });
  });

  it("counts a reach on the old side on the old numbers", () => {
    // The side comes from the `+` that was pressed, and a removed line is only
    // ever on one of them — including in a file the change deleted whole.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.reach("old", 40, 43));

    expect(result.current.covers({ old: removed(41), new: null })).toBe(true);
    expect(result.current.covers({ old: null, new: added(41) })).toBe(false);
    act(() => result.current.open("old", 40));
    expect(result.current.composing).toEqual({ side: "old", from: 40, to: 43 });
  });

  it("marks the lines reached so far, the way a drag does", () => {
    // Without this the reader is choosing a passage blind, and the arrows say
    // nothing until the box is already open.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.reach("new", 3, 5));

    expect([2, 3, 4, 5, 6].map((n) => result.current.covers(sidesOf(added(n))))).toEqual([
      false,
      true,
      true,
      true,
      false,
    ]);
  });

  it("leaves a span reached from another plus to that plus", () => {
    // Every `+` in the file answers to the same selection. Opening one over a
    // span anchored somewhere else would comment on lines nobody chose from
    // here.
    const { result } = renderHook(() => useLineSelection());
    act(() => result.current.reach("new", 3, 5));

    act(() => result.current.open("new", 9));

    expect(result.current.composing).toEqual({ side: "new", from: 9, to: 9 });
  });

  it("gives up a span that was reached and walked away from", () => {
    // Tabbing on leaves the rows lit otherwise, under a reader who is no
    // longer choosing anything.
    const { result } = renderHook(() => useLineSelection());
    act(() => result.current.reach("new", 3, 5));

    act(() => result.current.drop());

    expect(result.current.covers(sidesOf(added(4)))).toBe(false);
  });

  it("does not give up the span a drag is still holding", () => {
    // The `+` inside the gutter cell blurs while the pointer is going down the
    // numbers, and the drag is what owns the span until the mouse comes up.
    const { result } = renderHook(() => useLineSelection());
    act(() => result.current.start("new", 3));
    act(() => result.current.extend(added(5)));

    act(() => result.current.drop());

    expect(result.current.covers(sidesOf(added(4)))).toBe(true);
  });
});
