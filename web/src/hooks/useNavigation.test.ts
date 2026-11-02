import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { useNavigation } from "./useNavigation";
import { file } from "@/review/diff/testing";
import type { ReviewView } from "@/api";

it("goes to the next unread ahead, and wraps to the review's first unread past the end", () => {
  // Wrapping round is the server's answer, not a search from the top: the
  // file to go back to is the same first-unread the page landed on.
  const review = {
    blocks: [
      {
        slug: "core",
        title: "Core",
        context: "",
        lenses: [],
        files: [
          file({ path: "a.rs", viewed: false }),
          file({ path: "b.rs", viewed: true }),
          file({ path: "c.rs", viewed: false }),
        ],
        totalFiles: 3,
        viewedFiles: 1,
      },
    ],
    lenses: [],
    firstUnread: "a.rs",
  } as unknown as ReviewView;
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
