import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { useNavigation } from "./useNavigation";
import { block, file, review as reviewOf } from "@/review/diff/testing";

it("goes to the next unread ahead, and wraps to the review's first unread past the end", () => {
  // Wrapping round is the server's answer, not a search from the top: the
  // file to go back to is the same first-unread the page landed on.
  const review = reviewOf(
    [block("core", [file({ path: "a.rs" }), file({ path: "b.rs", viewed: true }), file({ path: "c.rs" })])],
    { firstUnread: "a.rs" },
  );
  const setCurrent = vi.fn();
  const nav = (current: string) => {
    const { result } = renderHook(() =>
      useNavigation({
        review,
        current,
        setCurrent,
        toggleViewed: vi.fn(),
        toggleSeen: vi.fn(),
        files: { isOpen: () => true, set: vi.fn(), keep: vi.fn() },
        comments: { expand: vi.fn(), drifted: [] },
      }),
    );
    return result.current;
  };

  const onA = nav("a.rs");
  act(() => onA.nextUnread());
  expect(setCurrent).toHaveBeenLastCalledWith("c.rs");

  const onC = nav("c.rs");
  act(() => onC.nextUnread());
  expect(setCurrent).toHaveBeenLastCalledWith("a.rs");
});
