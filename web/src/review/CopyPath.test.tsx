import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CopyPath } from "./CopyPath";

/** jsdom has no clipboard, so one is put there and watched. */
function clipboard(writeText = vi.fn().mockResolvedValue(undefined)) {
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText },
    configurable: true,
  });
  return writeText;
}

describe("copying a path", () => {
  beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
  afterEach(() => vi.useRealTimers());

  it("puts the whole path on the clipboard, not the file name", async () => {
    // The point of it: pasting into a terminal or a message, where the
    // directory is most of the value.
    const writeText = clipboard();
    render(<CopyPath path="src/map/domain/review_map/mod.rs" />);

    fireEvent.click(screen.getByRole("button"));

    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith("src/map/domain/review_map/mod.rs"),
    );
  });

  it("confirms, and takes the confirmation back down", async () => {
    clipboard();
    render(<CopyPath path="a.rs" />);

    fireEvent.click(screen.getByRole("button"));

    await screen.findByLabelText("Path copied");
    vi.advanceTimersByTime(2000);
    await waitFor(() => expect(screen.getByLabelText("Copy path")).toBeTruthy());
  });

  it("says nothing when the clipboard refuses", async () => {
    // A tick for a copy that did not happen is worse than no tick: the paste
    // is what finds out, and by then the path is gone from the screen.
    clipboard(vi.fn().mockRejectedValue(new Error("denied")));
    render(<CopyPath path="a.rs" />);

    fireEvent.click(screen.getByRole("button"));

    await waitFor(() => expect(screen.getByLabelText("Copy path")).toBeTruthy());
    expect(screen.queryByLabelText("Path copied")).toBeNull();
  });
});
