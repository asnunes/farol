import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
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
};

it("hides and shows a file's lens with the eye, keeping its name", () => {
  render(<FileLenses file={file({ lenses: [tests] })} onCite={() => {}} />);
  expect(screen.getByText(/tried again/)).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Hide Tests" }));
  expect(screen.queryByText(/tried again/)).toBeNull();
  expect(screen.getByText("Tests")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Show Tests" }));
  expect(screen.getByText(/tried again/)).toBeTruthy();
});

it("says which lens asks for the file to be read by a person", () => {
  render(<FileLenses file={file({ readYourself: "Risk" })} onCite={() => {}} />);
  expect(screen.getByText("Read this yourself (Risk)")).toBeTruthy();
});

it("draws nothing for a file no lens speaks about", () => {
  const { container } = render(<FileLenses file={file()} onCite={() => {}} />);
  expect(container.innerHTML).toBe("");
});
