import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useReview } from "./useReview";
import { api, type ReviewView } from "@/api";

beforeEach(() => {
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

it("says a mark that did not take, and answers false instead of throwing", async () => {
  // Thrown, the failure went nowhere: the page only `void`s the call.
  const review = { blocks: [], lenses: [], firstUnread: null } as unknown as ReviewView;
  vi.spyOn(api, "review").mockResolvedValue(review);
  vi.spyOn(api, "setViewed").mockRejectedValue(new Error("the store is unreadable"));
  const onError = vi.fn();
  const { result } = renderHook(() => useReview(onError));
  await waitFor(() => expect(result.current.review).not.toBeNull());

  let took: boolean | undefined;
  await act(async () => {
    took = await result.current.toggleViewed("a.rs", true);
  });

  expect(took).toBe(false);
  expect(onError).toHaveBeenCalledWith("the store is unreadable");
});
