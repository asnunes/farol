import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { FileLenses } from "./FileLenses";
import { file } from "@/review/diff/testing";
import type { LensView } from "@/api";

const tests: LensView = {
  id: "tests",
  title: "Tests",
  state: "written",
  text: "- a failure inside the window is tried again",
  writtenAt: "a1b2c3d",
  changed: 0,
  citations: [],
  target: { kind: "file", path: "src/a.rs" },
  seen: false,
};

const none = () => {};

it("hides and shows a file's lens with the eye, keeping its name", () => {
  render(<FileLenses file={file({ lenses: [tests] })} open={false} onToggleOpen={none} onCite={none} />);
  expect(screen.getByText(/tried again/)).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Hide Tests" }));
  expect(screen.queryByText(/tried again/)).toBeNull();
  expect(screen.getByText("Tests")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Show Tests" }));
  expect(screen.getByText(/tried again/)).toBeTruthy();
});

it("says why the map puts the file in focus", () => {
  render(<FileLenses file={file({ focus: "the whole retry rule" })} open={false} onToggleOpen={none} onCite={none} />);
  expect(screen.getByText("Focus: the whole retry rule")).toBeTruthy();
});

it("draws nothing for a file no lens speaks about", () => {
  const { container } = render(<FileLenses file={file()} open={false} onToggleOpen={none} onCite={none} />);
  expect(container.innerHTML).toBe("");
});

it("opens the code from under the lenses, which carry no tick of their own", () => {
  const toggle = vi.fn();
  render(<FileLenses file={file({ lenses: [tests] })} open={false} onToggleOpen={toggle} onCite={none} />);

  expect(screen.queryByRole("checkbox"), "the file's tick is the one that says they were read").toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Show code" }));
  expect(toggle).toHaveBeenCalled();
});

it("folds away with the code once the file is marked read", () => {
  const { container } = render(
    <FileLenses file={file({ lenses: [tests], viewed: true })} open={false} onToggleOpen={none} onCite={none} />,
  );
  expect(container.innerHTML).toBe("");
});
