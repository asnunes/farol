import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { ProseThreads } from "./ProseThreads";
import { comment, noComments } from "@/review/diff/testing";

it("starts a thread on a lens, with no comment for the pull request on offer", async () => {
  const actions = { ...noComments(), pendingOffered: true };
  render(
    <ProseThreads
      about={{ lens: "overview", target: { kind: "pr" } }}
      label="Thread on the Overview lens"
      threads={[]}
      actions={actions}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Comment" }));
  expect(screen.getByLabelText("Thread on the Overview lens")).toBeTruthy();
  expect(
    screen.queryByRole("button", { name: "Add as pending comment" }),
    "GitHub takes a review comment only on a line of the diff",
  ).toBeNull();

  fireEvent.change(screen.getByLabelText("Thread on the Overview lens"), { target: { value: "This misses the cache." } });
  fireEvent.click(screen.getByRole("button", { name: "Comment" }));
  expect(actions.add).toHaveBeenCalledWith({ lens: "overview", target: { kind: "pr" } }, "This misses the cache.");
});

it("still shows a lens's threads where no new one can be started", () => {
  render(
    <ProseThreads
      about={null}
      label="Thread on the Overview lens"
      threads={[comment({ id: "t1", body: "Still open." })]}
      actions={noComments()}
    />,
  );

  expect(screen.getByText("Still open.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Comment" }), "a folded or empty lens starts no thread").toBeNull();
});

it("draws a thread on a lens with no place label and no gutter indent", () => {
  // It sits right under the lens, so a word standing in for a place, and the
  // indent that clears a gutter of line numbers, only pushed the question away.
  const onLens = {
    ...comment({ id: "t2", body: "Explain this point." }),
    about: { kind: "lens" as const, key: "pr:overview", file: null },
  };
  const { container } = render(<ProseThreads about={null} label="Thread on the Overview lens" threads={[onLens]} actions={noComments()} />);

  expect(container.querySelector(".lbl")).toBeNull();
  expect(container.querySelector(".comment")?.className).not.toContain("pl-8");
});
