import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { StopReview } from "./StopReview";

it("stops the server only once the reviewer confirms", async () => {
  const stop = vi.fn().mockResolvedValue(undefined);
  render(<StopReview branch="feature/x" onStop={stop} />);

  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  expect(screen.getByText("Stop this review?")).toBeTruthy();
  expect(stop, "opening the dialog stops nothing").not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Stop review" }));
  await waitFor(() => expect(stop).toHaveBeenCalledTimes(1));
});

it("keeps the dialog open with the reason when the server will not stop", async () => {
  render(<StopReview branch="feature/x" onStop={vi.fn().mockRejectedValue(new Error("still writing"))} />);

  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  fireEvent.click(screen.getByRole("button", { name: "Stop review" }));

  expect(await screen.findByText(/still writing/)).toBeTruthy();
});
