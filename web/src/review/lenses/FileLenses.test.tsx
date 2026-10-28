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

it("hides and shows a file's lens with the eye, keeping its name", () => {
  render(<FileLenses file={file({ lenses: [tests] })} onCite={() => {}} onSeen={() => {}} />);
  expect(screen.getByText(/tried again/)).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Hide Tests" }));
  expect(screen.queryByText(/tried again/)).toBeNull();
  expect(screen.getByText("Tests")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Show Tests" }));
  expect(screen.getByText(/tried again/)).toBeTruthy();
});

it("says which lens asks for the file to be read by a person", () => {
  render(<FileLenses file={file({ readYourself: "Risk" })} onCite={() => {}} onSeen={() => {}} />);
  expect(screen.getByText("Read this yourself (Risk)")).toBeTruthy();
});

it("draws nothing for a file no lens speaks about", () => {
  const { container } = render(<FileLenses file={file()} onCite={() => {}} onSeen={() => {}} />);
  expect(container.innerHTML).toBe("");
});

it("starts a lens already seen hidden, and marks one seen with its tick", () => {
  const seen = vi.fn();
  render(<FileLenses file={file({ lenses: [{ ...tests, seen: true }] })} onCite={() => {}} onSeen={seen} />);

  expect(screen.queryByText(/tried again/)).toBeNull();
  fireEvent.click(screen.getByRole("checkbox", { name: "Mark Tests as seen" }));
  expect(seen).toHaveBeenCalledWith(expect.objectContaining({ id: "tests" }), false);
});
