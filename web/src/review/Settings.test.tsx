import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { api, type Config } from "@/api";
import { Settings } from "./Settings";

afterEach(() => vi.restoreAllMocks());

it("renders backend labels and sends the supplied key without knowing the setting", async () => {
  const read = vi.spyOn(api, "config").mockResolvedValue(config(true));
  const save = vi.spyOn(api, "setSetting").mockResolvedValue(config(false));
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  const checkbox = await screen.findByRole("checkbox", { name: "Example preference" });
  expect((checkbox as HTMLInputElement).checked).toBe(true);
  expect(screen.getByText("An explanation from the server.")).toBeTruthy();
  fireEvent.click(checkbox);
  await screen.findByText("Saved");
  expect(save).toHaveBeenCalledWith("example_preference", false);
  expect((checkbox as HTMLInputElement).checked).toBe(false);
  fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
  read.mockResolvedValue(config(true));
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  await waitFor(() => expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true));
});

it("keeps the saved value visible when saving fails", async () => {
  vi.spyOn(api, "config").mockResolvedValue(config(true));
  vi.spyOn(api, "setSetting").mockRejectedValue(new Error("Cannot write config"));
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  fireEvent.click(await screen.findByRole("checkbox"));
  expect((await screen.findByRole("alert")).textContent).toBe("Cannot write config");
  expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
});

it("hides settings when the server supplies no fields", async () => {
  vi.spyOn(api, "config").mockResolvedValue({ settings: [] });
  await act(async () => { render(<Settings />); });
  expect(screen.queryByRole("button", { name: "Settings" })).toBeNull();
  expect(screen.queryByRole("checkbox")).toBeNull();
});

it("renders multiple boolean settings and omits absent descriptions", async () => {
  const answer = config(true);
  answer.settings.push({ ...answer.settings[0], key: "second", title: "Another preference", description: null, value: false });
  vi.spyOn(api, "config").mockResolvedValue(answer);
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  expect(screen.getAllByRole("checkbox")).toHaveLength(2);
  expect(screen.getByRole("checkbox", { name: "Another preference" }).hasAttribute("aria-describedby")).toBe(false);
});

function config(value: boolean): Config {
  return { settings: [{
    key: "example_preference",
    title: "Example preference",
    description: "An explanation from the server.",
    type: "boolean",
    default: true,
    value,
  }] };
}
