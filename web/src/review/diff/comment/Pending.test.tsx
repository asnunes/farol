import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { Pending } from "./Pending";
import { noComments } from "../testing";

it("a pending comment with no thread goes by its own id, and offers nothing to ask", () => {
  // Written on the pull request's page, or left by a closed thread: there is
  // no conversation to send the agent, only the comment to change or take back.
  const actions = { ...noComments(), agent: "Codex", pendingOffered: true };
  render(
    <Pending
      id="PRRC_9"
      thread={null}
      remote={{ state: "pending", body: "Rename it.", url: null }}
      actions={actions}
      busy={false}
    />,
  );

  expect(screen.getByText("Rename it.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: /Ask Codex/ })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Remove" }));
  expect(actions.removePending).toHaveBeenCalledWith("PRRC_9");
});
