import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { usePublishing } from "./usePublishing";

afterEach(() => vi.unstubAllGlobals());

describe("asking whether the review can be sent", () => {
  it("asks once on load", async () => {
    const asked = serve({ state: "ready", branch: "feature/x", pullRequest: 12 });
    const { result } = renderHook(() => usePublishing());

    await waitFor(() => expect(result.current.readiness?.state).toBe("ready"));
    expect(asked).toHaveBeenCalledTimes(1);
  });

  it("asks again when the reader comes back, while the answer is still no", async () => {
    // The token was pasted into a file, or the branch was pushed, in the
    // terminal next door. Nothing about that reaches this page.
    const asked = serve({ state: "noToken", branch: "feature/x" });
    renderHook(() => usePublishing());
    await waitFor(() => expect(asked).toHaveBeenCalledTimes(1));

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });

    await waitFor(() => expect(asked).toHaveBeenCalledTimes(2));
  });

  it("stops asking once the answer is yes", async () => {
    // Nothing about returning to a tab can un-open a pull request, and the
    // alternative is a request to GitHub for every tab switch of the session.
    const asked = serve({ state: "ready", branch: "feature/x", pullRequest: 12 });
    const { result } = renderHook(() => usePublishing());
    await waitFor(() => expect(result.current.readiness?.state).toBe("ready"));

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      document.dispatchEvent(new Event("visibilitychange"));
    });

    expect(asked).toHaveBeenCalledTimes(1);
  });

  it("treats a burst of focus events as one return", async () => {
    // Coming back to a tab fires focus and visibilitychange together, and a
    // window manager can fire focus several times over.
    const asked = serve({ state: "noToken", branch: "feature/x" });
    renderHook(() => usePublishing());
    await waitFor(() => expect(asked).toHaveBeenCalledTimes(1));

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      document.dispatchEvent(new Event("visibilitychange"));
      window.dispatchEvent(new Event("focus"));
    });

    await waitFor(() => expect(asked).toHaveBeenCalledTimes(2));
    await new Promise((settle) => setTimeout(settle, 300));
    expect(asked).toHaveBeenCalledTimes(2);
  });

  it("keeps the last answer when the question cannot be asked", async () => {
    // A server that blinked is not an answer of no. Turning it into one would
    // send the reader off to fix a token that is perfectly good.
    const asked = serve({ state: "noToken", branch: "feature/x" });
    const { result } = renderHook(() => usePublishing());
    await waitFor(() => expect(result.current.readiness?.state).toBe("noToken"));

    asked.mockRejectedValueOnce(new Error("down"));
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });

    expect(result.current.readiness?.state).toBe("noToken");
  });
});

/** The readiness route, answering the same thing every time. */
function serve(readiness: unknown) {
  const asked = vi.fn(
    async () =>
      new Response(JSON.stringify(readiness), {
        headers: { "content-type": "application/json" },
      }),
  );
  vi.stubGlobal("fetch", asked);
  return asked;
}
