import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useGitHub } from "./useGitHub";
import { api, type PullRequestView } from "@/api";

describe("the pull request's state", () => {
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

  it("is asked again when the reader comes back from opening the pull request", async () => {
    // Opened on GitHub, in another tab: nothing local announces it, and
    // coming back is when the reader expects the page to know.
    const github = vi
      .spyOn(api, "github")
      .mockResolvedValueOnce(view("no-pull-request"))
      .mockResolvedValue(view("ready"));
    const { result } = renderHook(() => useGitHub(onError));
    await waitFor(() => expect(result.current.pull?.connection).toBe("no-pull-request"));

    leave();
    comeBack();

    await waitFor(() => expect(result.current.pull?.connection).toBe("ready"));
    expect(github).toHaveBeenCalledTimes(2);
  });

  it("is not asked again on every return once nothing is missing", async () => {
    const github = vi.spyOn(api, "github").mockResolvedValue(view("ready"));
    const { result } = renderHook(() => useGitHub(onError));
    await waitFor(() => expect(result.current.pull?.connection).toBe("ready"));

    leave();
    comeBack();

    expect(github).toHaveBeenCalledTimes(1);
  });
});

const onError = () => {};

function leave() {
  act(() => {
    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
}

function comeBack() {
  act(() => {
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
}

function view(connection: PullRequestView["connection"]): PullRequestView {
  return {
    connection,
    number: connection === "ready" ? 7 : null,
    mine: false,
    author: connection === "ready" ? "alice" : null,
    url: connection === "ready" ? "https://github.com/o/r/pull/7" : null,
    openAt: null,
    login: "gh auth login --hostname github.com",
    numbers: [],
    mode: { mode: "peer", reason: "chosen by you" },
    pending: {},
    pendingFailed: null,
  };
}
