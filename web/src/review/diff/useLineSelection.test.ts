import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useLineSelection } from "./useLineSelection";

/** Let go of the mouse, wherever the pointer is. */
function release() {
  act(() => {
    window.dispatchEvent(new MouseEvent("mouseup"));
  });
}

describe("picking the lines a comment is about", () => {
  it("covers the span that was dragged", () => {
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start(3));
    act(() => result.current.extend(5));
    release();

    expect(result.current.composing).toEqual({ from: 3, to: 5 });
  });

  it("reads a drag upward as the same span as a drag downward", () => {
    // The reader drags from the line that puzzled them, which is as often the
    // last line of the passage as the first.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start(5));
    act(() => result.current.extend(3));
    release();

    expect(result.current.composing).toEqual({ from: 3, to: 5 });
  });

  it("takes a press and a release on one number as that one line", () => {
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start(7));
    release();

    expect(result.current.composing).toEqual({ from: 7, to: 7 });
  });

  it("marks the lines under the pointer while the drag is under way", () => {
    // Without this the reader is choosing a passage blind and only finds out
    // what they picked once the box is open.
    const { result } = renderHook(() => useLineSelection());

    act(() => result.current.start(3));
    act(() => result.current.extend(5));

    expect([2, 3, 4, 5, 6].map(result.current.covers)).toEqual([false, true, true, true, false]);
  });

  it("stops marking once the box is open", () => {
    const { result } = renderHook(() => useLineSelection());
    act(() => result.current.start(3));
    act(() => result.current.extend(5));

    release();

    expect(result.current.covers(4)).toBe(false);
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
    act(() => result.current.open(9));

    act(() => result.current.close());

    expect(result.current.composing).toBeNull();
  });
});
