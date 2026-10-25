import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { DriftedThreads } from "./DriftedThreads";
import { comment, noComments } from "@/review/diff/testing";

describe("threads off the diff", () => {
  it("are answered like any other, without a comment for the pull request", () => {
    // GitHub takes a review comment only on a line of the diff.
    const actions = { ...noComments(), agent: "Codex", pendingOffered: true };
    render(<DriftedThreads threads={[comment({ id: "t1" })]} actions={actions} />);

    expect(screen.getByRole("button", { name: "Reply" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Generate pending comment" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Use as pending comment" })).toBeNull();
    expect(screen.getByText(/No longer on the diff/)).toBeTruthy();
  });

  it("draw nothing when there are none", () => {
    const { container } = render(<DriftedThreads threads={[]} actions={noComments()} />);

    expect(container.textContent).toBe("");
  });
});
