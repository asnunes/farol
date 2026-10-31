import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Drift } from "./Drift";
import type { DriftView } from "@/api";

it("offers to push your own branch, and pushes only once confirmed", async () => {
  const onPush = vi.fn().mockResolvedValue(true);
  show({ kind: "unpushed", commits: 2 }, true, onPush);

  fireEvent.click(screen.getByRole("button", { name: "Push 2 commits" }));
  expect(onPush, "pushing publishes code, so it asks first").not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Push" }));
  await vi.waitFor(() => expect(onPush).toHaveBeenCalled());
});

it("never offers to push somebody else's branch", () => {
  show({ kind: "unpushed", commits: 2 }, false);
  expect(screen.getByRole("img", { name: "2 commits not on the pull request yet" })).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
});

it("asks for a pull when the pull request has commits this checkout does not", () => {
  show({ kind: "newer", commits: null }, true);
  expect(screen.getByRole("img", { name: /pull to read them/ })).toBeTruthy();
  expect(screen.queryByRole("button")).toBeNull();
});

function show(drift: DriftView, mine: boolean, onPush = vi.fn()) {
  return render(
    <TooltipProvider>
      <Drift drift={drift} mine={mine} number={87} branch="feat/x" onPush={onPush} />
    </TooltipProvider>,
  );
}
