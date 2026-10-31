import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { Drift } from "./Drift";

it("offers to push your own branch, and pushes only once confirmed", async () => {
  const onPush = vi.fn().mockResolvedValue(true);
  render(<Drift drift={{ kind: "unpushed", commits: 2 }} mine={true} number={87} branch="feat/x" onPush={onPush} />);

  fireEvent.click(screen.getByRole("button", { name: "Push 2 commits" }));
  expect(onPush, "pushing publishes code, so it asks first").not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Push" }));
  await vi.waitFor(() => expect(onPush).toHaveBeenCalled());
});

it("never offers to push somebody else's branch", () => {
  render(<Drift drift={{ kind: "unpushed", commits: 2 }} mine={false} number={87} branch="feat/x" onPush={vi.fn()} />);
  expect(screen.getByText("2 commits unpushed")).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
});

it("asks for a pull when the pull request has commits this checkout does not", () => {
  render(<Drift drift={{ kind: "newer", commits: null }} mine={true} number={87} branch="feat/x" onPush={vi.fn()} />);
  expect(screen.getByText("Pull request is ahead: pull")).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
});
