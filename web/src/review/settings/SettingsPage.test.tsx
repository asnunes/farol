import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { api, type BooleanSetting, type Config, type LensRow, type TextSetting } from "@/api";
import { SettingsPage } from "./SettingsPage";
import type { SettingsSection } from "@/hooks/useSettingsRoute";

const overview: LensRow = { id: "overview", title: "Overview", group: "pr", prompt: "Say it.", enabled: true };

beforeEach(() => {
  vi.spyOn(api, "config").mockResolvedValue(config("Answer it."));
  vi.spyOn(api, "lenses").mockResolvedValue({ lenses: [overview] });
});
afterEach(() => vi.restoreAllMocks());

it("saves a switch as it is flipped, with no bar to press", async () => {
  const save = vi.spyOn(api, "setSetting").mockResolvedValue(config("Answer it.", false));
  show("general");

  fireEvent.click(await screen.findByRole("switch", { name: "Open browser automatically" }));

  await waitFor(() => expect(save).toHaveBeenCalledWith("open_browser", false));
  expect(screen.queryByRole("region", { name: "Unsaved changes" })).toBeNull();
});

it("holds an edited instruction until the bar's Save, and Discard puts it back", async () => {
  const save = vi.spyOn(api, "setSetting").mockResolvedValue(config("Only answer."));
  show("instructions");
  const field = await screen.findByRole("textbox", { name: "Instruction sent with each comment" });

  fireEvent.change(field, { target: { value: "Only answer." } });
  expect(screen.getByText("1 unsaved change")).toBeTruthy();
  expect(save, "nothing is written while typing").not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Discard" }));
  expect((field as HTMLTextAreaElement).value).toBe("Answer it.");

  fireEvent.change(field, { target: { value: "Only answer." } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(screen.queryByRole("region", { name: "Unsaved changes" })).toBeNull());
  expect(save).toHaveBeenCalledWith("send_instruction", "Only answer.");
});

it("restores a default as a draft, and sends it as a reset on Save", async () => {
  const reset = vi.spyOn(api, "resetSetting").mockResolvedValue(config("The default."));
  show("instructions");

  fireEvent.click(await screen.findByRole("button", { name: "Reset to default" }));
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("The default.");
  expect(reset).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(reset).toHaveBeenCalledWith("send_instruction"));
});

it("keeps a draft the server refused, with the reason under it", async () => {
  vi.spyOn(api, "setSetting").mockRejectedValue(new Error("Cannot write config"));
  show("instructions");

  fireEvent.change(await screen.findByRole("textbox"), { target: { value: "Only answer." } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));

  expect((await screen.findByRole("alert")).textContent).toBe("Cannot write config");
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("Only answer.");
  expect(screen.getByText("1 unsaved change")).toBeTruthy();
});

it("flips a lens at once, but holds its words, a new lens and a deletion for Save", async () => {
  const change = vi.spyOn(api, "saveLens").mockResolvedValue({ lenses: [overview] });
  const add = vi.spyOn(api, "addLens").mockResolvedValue({ lenses: [overview] });
  const remove = vi.spyOn(api, "deleteLens").mockResolvedValue({ lenses: [] });
  show("lenses");

  fireEvent.click(await screen.findByRole("switch", { name: "Overview is on" }));
  await waitFor(() => expect(change).toHaveBeenCalledWith("overview", { enabled: false }));

  fireEvent.click(screen.getByRole("button", { name: "Edit" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), { target: { value: "Summary" } });
  fireEvent.click(screen.getAllByRole("button", { name: "Add lens" })[2]);
  fireEvent.change(screen.getAllByRole("textbox", { name: "Title" })[1], { target: { value: "Migrations" } });
  fireEvent.change(screen.getAllByRole("textbox", { name: /Prompt/ })[1], { target: { value: "What each does." } });
  expect(screen.getByText("2 unsaved changes")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(add).toHaveBeenCalledWith({ group: "file", title: "Migrations", prompt: "What each does." }));
  expect(change).toHaveBeenCalledWith("overview", { title: "Summary", prompt: "Say it." });

  fireEvent.click(screen.getByRole("button", { name: "Delete" }));
  expect(remove, "a deletion waits for Save too").not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(remove).toHaveBeenCalledWith("overview"));
});

it("asks before leaving with something unsaved", async () => {
  const guard = { current: () => true };
  render(<SettingsPage section="instructions" onSection={vi.fn()} onClose={vi.fn()} guard={guard} />);
  fireEvent.change(await screen.findByRole("textbox"), { target: { value: "Only answer." } });
  const ask = vi.spyOn(window, "confirm").mockReturnValue(false);

  expect(guard.current()).toBe(false);
  expect(ask).toHaveBeenCalled();
});

function show(section: SettingsSection) {
  return render(<SettingsPage section={section} onSection={vi.fn()} onClose={vi.fn()} guard={{ current: () => true }} />);
}

function config(instruction: string, browser = true): Config {
  const open: BooleanSetting = {
    key: "open_browser",
    title: "Open browser automatically",
    description: null,
    type: "boolean",
    default: true,
    value: browser,
  };
  const send: TextSetting = {
    key: "send_instruction",
    title: "Instruction sent with each comment",
    description: null,
    type: "text",
    default: "The default.",
    value: instruction,
  };
  return { settings: [open, send] };
}
