import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { ProseThreads } from "./ProseThreads";
import { noComments } from "@/review/diff/testing";

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
