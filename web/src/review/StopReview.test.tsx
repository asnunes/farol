import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { StopReview } from "./StopReview";
import { api } from "@/api";

it("stops the server only once the reviewer confirms", async () => {
  const stop = vi.spyOn(api, "stop").mockResolvedValue(undefined);
  const onStopped = vi.fn();
  render(<StopReview branch="feature/x" onStopped={onStopped} />);

  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  expect(screen.getByText("Stop this review?")).toBeTruthy();
  expect(stop, "opening the dialog stops nothing").not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Stop review" }));
  await waitFor(() => expect(onStopped).toHaveBeenCalled());
  expect(stop).toHaveBeenCalledTimes(1);
});
