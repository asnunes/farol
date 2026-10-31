import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { Drift } from "./Drift";

it("offers to push your own branch, and pushes only once confirmed", async () => {
  const onPush = vi.fn().mockResolvedValue(true);
  render(<Drift drift={{ kind: "unpushed", commits: 2 }} mine={true} number={87} branch="feat/x" onPush={onPush} />);

  expect(screen.getByText("2 commits not on the pull request")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Push" }));
  expect(onPush, "pushing publishes code, so it asks first").not.toHaveBeenCalled();

  fireEvent.click(screen.getAllByRole("button", { name: "Push" }).at(-1)!);
  await vi.waitFor(() => expect(onPush).toHaveBeenCalled());
});

it("never offers to push somebody else's branch", () => {
  render(<Drift drift={{ kind: "unpushed", commits: 2 }} mine={false} number={87} branch="feat/x" onPush={vi.fn()} />);
  expect(screen.queryByRole("button", { name: "Push" })).toBeNull();
});

it("asks for a pull when the pull request has commits this checkout does not", () => {
  render(<Drift drift={{ kind: "newer", commits: null }} mine={true} number={87} branch="feat/x" onPush={vi.fn()} />);
  expect(screen.getByText(/pull to read them/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Push" })).toBeNull();
});
