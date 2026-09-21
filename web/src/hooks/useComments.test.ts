import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useComments } from "./useComments";
import { api } from "@/api";
import type { CommentView } from "@/api";

describe("a file's comments", () => {
  beforeEach(() => {
    // The hook subscribes on mount; without a stub jsdom throws.
    vi.stubGlobal(
      "EventSource",
      class {
        addEventListener() {}
        removeEventListener() {}
        close() {}
      },
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("is grouped in one place, so the count and the boxes cannot disagree", async () => {
    // What this replaces: the sidebar, the file header and the diff each
    // filtered the whole list by path on their own. Four narrowings of one
    // question is four chances for the margin to claim a comment the code
    // below it never draws.
    const { on } = await loaded([
      comment("1", "src/a.rs"),
      comment("2", "src/b.rs"),
      comment("3", "src/a.rs"),
    ]);

    expect(on("src/a.rs").map((c) => c.id)).toEqual(["1", "3"]);
    expect(on("src/b.rs").map((c) => c.id)).toEqual(["2"]);
  });

  it("hands back the same empty list every time for a file with none", async () => {
    // A fresh array per render would remount every row that has no comment.
    const { on } = await loaded([comment("1", "src/a.rs")]);

    expect(on("src/quiet.rs")).toHaveLength(0);
    expect(on("src/quiet.rs")).toBe(on("src/other.rs"));
  });

  /** The hook over a list the server answered with. */
  async function loaded(comments: CommentView[]) {
    vi.spyOn(api, "comments").mockResolvedValue({ comments, unreadable: [] });
    const { result } = renderHook(() => useComments(vi.fn()));
    await waitFor(() => expect(result.current.comments).toHaveLength(comments.length));
    return result.current;
  }

  function comment(id: string, path: string): CommentView {
    return { id, path, side: "new", from: 1, to: 1, body: "why?", published: null };
  }
});
