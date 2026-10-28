import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { api, type BooleanSetting, type Config, type TextSetting } from "@/api";
import { Settings } from "./Settings";

// The lenses have tests of their own; here they stay out of the way.
beforeEach(() => vi.spyOn(api, "lenses").mockResolvedValue({ lenses: [] }));
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
  answer.settings.push({ ...(answer.settings[0] as BooleanSetting), key: "second", title: "Another preference", description: null, value: false });
  vi.spyOn(api, "config").mockResolvedValue(answer);
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  expect(screen.getAllByRole("checkbox")).toHaveLength(2);
  expect(screen.getByRole("checkbox", { name: "Another preference" }).hasAttribute("aria-describedby")).toBe(false);
});

it("edits a text setting as a draft and writes it only on Save", async () => {
  const answer: Config = { settings: [instruction("Answer it.")] };
  vi.spyOn(api, "config").mockResolvedValue(answer);
  const save = vi.spyOn(api, "setSetting").mockResolvedValue({ settings: [instruction("Only answer.")] });
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  const box = await screen.findByRole("textbox", { name: "Instruction sent with each comment" });
  const saveButton = screen.getByRole("button", { name: "Save" });
  expect((saveButton as HTMLButtonElement).disabled).toBe(true);

  fireEvent.change(box, { target: { value: "Only answer." } });
  expect(save).not.toHaveBeenCalled();
  fireEvent.click(saveButton);

  await screen.findByText("Saved");
  expect(save).toHaveBeenCalledWith("send_instruction", "Only answer.");
  expect((box as HTMLTextAreaElement).value).toBe("Only answer.");
});

it("restores a text setting's default through the server", async () => {
  const read = vi.spyOn(api, "config").mockResolvedValue({ settings: [instruction("Only answer.")] });
  const reset = vi.spyOn(api, "resetSetting").mockResolvedValue({ settings: [instruction("Answer it.")] });
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  // Opening reads the configuration again; let that answer land first, or it
  // arrives after the reset and paints the old value back.
  await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
  await act(async () => {});
  fireEvent.click(await screen.findByRole("button", { name: "Restore default" }));

  await screen.findByText("Saved");
  expect(reset).toHaveBeenCalledWith("send_instruction");
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("Answer it.");
});

it("keeps the draft when saving a text setting fails", async () => {
  vi.spyOn(api, "config").mockResolvedValue({ settings: [instruction("Answer it.")] });
  vi.spyOn(api, "setSetting").mockRejectedValue(new Error("Cannot write config"));
  render(<Settings />);
  fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
  const box = await screen.findByRole("textbox");
  fireEvent.change(box, { target: { value: "Mine." } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));

  expect((await screen.findByRole("alert")).textContent).toBe("Cannot write config");
  expect((box as HTMLTextAreaElement).value).toBe("Mine.");
});

function instruction(value: string): TextSetting {
  return {
    key: "send_instruction",
    title: "Instruction sent with each comment",
    description: null,
    type: "text",
    default: "Answer it.",
    value,
  };
}

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
