import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { LensText } from "./LensText";
import type { CitationView } from "@/api";

const checked: CitationView = { text: "src/retry.rs:90-91", path: "src/retry.rs", side: "new", from: 90, to: 91 };

it("turns a checked citation into a way to the code, and leaves the rest as code", () => {
  const cite = vi.fn();
  render(<LensText text="Tried again `src/retry.rs:90-91`; see `AGENTS.md:3`." citations={[checked]} onCite={cite} />);

  fireEvent.click(screen.getByRole("button", { name: "src/retry.rs:90-91" }));

  expect(cite).toHaveBeenCalledWith(checked);
  expect(screen.queryByRole("button", { name: "AGENTS.md:3" })).toBeNull();
  expect(screen.getByText("AGENTS.md:3").tagName).toBe("CODE");
});

it("names only the lines inside the file's own lens", () => {
  render(<LensText text="`src/retry.rs:90-91`" citations={[checked]} inFile="src/retry.rs" onCite={() => {}} />);

  expect(screen.getByRole("button", { name: "lines 90–91" })).toBeTruthy();
});
