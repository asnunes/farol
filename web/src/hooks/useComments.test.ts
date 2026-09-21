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

  it("are looked up under their path, not filtered out of a flat list", async () => {
    const on = await loaded({
      "src/a.rs": [comment("1"), comment("3")],
      "src/b.rs": [comment("2")],
    });

    expect(on("src/a.rs").map((c) => c.id)).toEqual(["1", "3"]);
    expect(on("src/b.rs").map((c) => c.id)).toEqual(["2"]);
  });

  it("hand back the same empty list every time for a file with none", async () => {
    // A fresh array per render would remount every row that has no comment.
    const on = await loaded({ "src/a.rs": [comment("1")] });

    expect(on("src/quiet.rs")).toHaveLength(0);
    expect(on("src/quiet.rs")).toBe(on("src/other.rs"));
  });

  /** The hook over the answer the server gave.
   *
   * The error callback is made once, outside the render: the hook rebuilds its
   * loader whenever that identity changes, and the effect that fetches hangs
   * off the loader. A fresh `vi.fn()` per render would set state, re-render,
   * and go round until the heap gave out — which is what it did.
   */
  async function loaded(files: Record<string, CommentView[]>) {
    vi.spyOn(api, "comments").mockResolvedValue({ files, unreadable: [] });
    const onError = vi.fn();
    const { result } = renderHook(() => useComments(onError));
    const first = Object.keys(files)[0];
    await waitFor(() => expect(result.current.on(first)).not.toHaveLength(0));
    return result.current.on;
  }

  function comment(id: string): CommentView {
    return { id, path: "src/a.rs", side: "new", from: 1, to: 1, body: "why?", published: null };
  }
});
