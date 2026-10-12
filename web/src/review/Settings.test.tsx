import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { api } from "@/api";
import { Settings } from "./Settings";

afterEach(() => vi.restoreAllMocks());

it("saves the browser preference and reads it again when reopened", async () => {
  const read = vi.spyOn(api, "config").mockResolvedValue({ open_browser: true, show_open_browser: true });
  const save = vi.spyOn(api, "setOpenBrowser").mockResolvedValue(undefined);
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  const checkbox = await screen.findByRole("checkbox");
  expect((checkbox as HTMLInputElement).checked).toBe(true);
  fireEvent.click(checkbox);
  await screen.findByText("Saved");
  expect(save).toHaveBeenCalledWith(false);
  expect((checkbox as HTMLInputElement).checked).toBe(false);
  fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
  read.mockResolvedValue({ open_browser: true, show_open_browser: true });
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  await waitFor(() => expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true));
  expect(read).toHaveBeenCalled();
});

it("keeps the saved value visible when saving fails", async () => {
  vi.spyOn(api, "config").mockResolvedValue({ open_browser: true, show_open_browser: true });
  vi.spyOn(api, "setOpenBrowser").mockRejectedValue(new Error("Cannot write config"));
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  fireEvent.click(await screen.findByRole("checkbox"));
  expect((await screen.findByRole("alert")).textContent).toBe("Cannot write config");
  expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
});

it("hides settings when browser opening does not apply to the server", async () => {
  const read = vi.spyOn(api, "config").mockResolvedValue({
    open_browser: true,
    show_open_browser: false,
  });
  render(<Settings />);
  await waitFor(() => expect(read).toHaveBeenCalled());
  expect(screen.queryByRole("button", { name: "Settings" })).toBeNull();
  expect(screen.queryByRole("checkbox")).toBeNull();
});
