import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { api, type LensRow } from "@/api";
import { LensSettings } from "./LensSettings";

afterEach(() => vi.restoreAllMocks());

const overview: LensRow = { id: "overview", title: "Overview", group: "pr", prompt: "Say it.", enabled: true };
const tests: LensRow = { id: "tests", title: "Tests", group: "file", prompt: "List them.", enabled: true };

it("lists the lenses in their groups and switches one by sending only the switch", async () => {
  vi.spyOn(api, "lenses").mockResolvedValue({ lenses: [overview, tests] });
  const save = vi.spyOn(api, "saveLens").mockResolvedValue({ lenses: [{ ...overview, enabled: false }, tests] });
  render(<LensSettings open />);

  fireEvent.click(await screen.findByRole("checkbox", { name: "Overview is on" }));

  await waitFor(() => expect((screen.getByRole("checkbox", { name: "Overview is on" }) as HTMLInputElement).checked).toBe(false));
  expect(save).toHaveBeenCalledWith("overview", { enabled: false });
  expect(screen.getByText("Pull request")).toBeTruthy();
  expect(screen.getByText("File")).toBeTruthy();
});

it("adds a lens to the group it was asked in", async () => {
  vi.spyOn(api, "lenses").mockResolvedValue({ lenses: [] });
  const add = vi.spyOn(api, "addLens").mockResolvedValue({
    lenses: [{ id: "migrations", title: "Migrations", group: "file", prompt: "What each does.", enabled: true }],
  });
  render(<LensSettings open />);

  const buttons = await screen.findAllByRole("button", { name: "Add lens" });
  fireEvent.click(buttons[2]);
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), { target: { value: "Migrations" } });
  fireEvent.change(screen.getByRole("textbox", { name: /Prompt/ }), { target: { value: "What each does." } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));

  await screen.findByText("Migrations");
  expect(add).toHaveBeenCalledWith({ title: "Migrations", group: "file", prompt: "What each does." });
});
