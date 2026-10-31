import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { SyncReads } from "./SyncReads";

it("says what the sync did on the pull request", async () => {
  const onSync = vi.fn().mockResolvedValue({ number: 87, marked: 12, unmarked: 1 });
  render(<SyncReads onSync={onSync} />);

  fireEvent.click(screen.getByRole("button", { name: "Sync read marks to GitHub" }));

  expect(await screen.findByText("12 marked, 1 unmarked on #87")).toBeTruthy();
});

it("says nothing of its own when the sync is refused", async () => {
  // The refusal is the caller's to show, the way every other one is.
  const onSync = vi.fn().mockResolvedValue(null);
  render(<SyncReads onSync={onSync} />);

  fireEvent.click(screen.getByRole("button", { name: "Sync read marks to GitHub" }));

  await vi.waitFor(() => expect(onSync).toHaveBeenCalled());
  expect(screen.queryByRole("status")).toBeNull();
});
