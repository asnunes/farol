import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { SyncReads } from "./SyncReads";

it("tells what the sync did on the pull request, away from the bar", async () => {
  const onSync = vi.fn().mockResolvedValue({ number: 87, marked: 12, unmarked: 1 });
  const onSynced = vi.fn();
  render(<SyncReads onSync={onSync} onSynced={onSynced} blocked={false} synced={null} />);

  fireEvent.click(screen.getByRole("button", { name: "Sync read marks to GitHub" }));

  await vi.waitFor(() => expect(onSynced).toHaveBeenCalledWith("Read marks synced: 12 marked, 1 unmarked on #87"));
});

it("tells nothing when the sync is refused", async () => {
  // The refusal is the caller's to show, the way every other one is.
  const onSync = vi.fn().mockResolvedValue(null);
  const onSynced = vi.fn();
  render(<SyncReads onSync={onSync} onSynced={onSynced} blocked={false} synced={null} />);

  fireEvent.click(screen.getByRole("button", { name: "Sync read marks to GitHub" }));

  await vi.waitFor(() => expect(onSync).toHaveBeenCalled());
  expect(onSynced).not.toHaveBeenCalled();
});

it("cannot sync while the pull request shows another commit", () => {
  render(<SyncReads onSync={vi.fn()} onSynced={vi.fn()} blocked={true} synced={null} />);
  expect(screen.getByRole("button", { name: "Sync read marks to GitHub" }).hasAttribute("disabled")).toBe(true);
});

it("says whether GitHub already shows what was read here", () => {
  // Without it, the only way to know was to press and read what came back.
  const { rerender } = render(<SyncReads onSync={vi.fn()} onSynced={vi.fn()} blocked={false} synced={false} />);
  expect(screen.getByRole("button", { name: "Sync read marks to GitHub (not in sync)" })).toBeTruthy();

  rerender(<SyncReads onSync={vi.fn()} onSynced={vi.fn()} blocked={false} synced={true} />);
  expect(screen.getByRole("button", { name: "Sync read marks to GitHub (in sync)" })).toBeTruthy();
});
