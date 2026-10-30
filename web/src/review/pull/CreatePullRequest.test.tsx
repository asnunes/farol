import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CreatePullRequest } from "./CreatePullRequest";
import type { GitHubState } from "@/hooks/useGitHub";
import type { PullRequestView } from "@/api";

function pull(over: Partial<PullRequestView> = {}): PullRequestView {
  return {
    connection: "no-pull-request",
    number: null,
    mine: false,
    author: null,
    url: null,
    openAt: null,
    login: "gh auth login",
    numbers: [],
    mode: { mode: "self", reason: "no pull request to tell" },
    pending: {},
    pendingFailed: null,
    draft: null,
    ...over,
  };
}

function github(): GitHubState {
  return {
    pull: null,
    reload: vi.fn(),
    chooseMode: vi.fn(),
    submit: vi.fn(),
    openPull: vi.fn().mockResolvedValue({ number: 13, url: "https://example.test/pull/13" }),
    generatePullDraft: vi.fn().mockResolvedValue(true),
  };
}

it("pushes and opens a branch GitHub has not seen, with no Generate when no agent is connected", async () => {
  const gh = github();
  render(<CreatePullRequest pull={pull()} branch="feature/x" agent={null} github={gh} />);

  fireEvent.click(screen.getByRole("button", { name: "Create pull request…" }));
  expect(screen.getByText(/farol pushes it to origin first/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: /Generate/ }), "Generate needs an agent").toBeNull();

  fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Stop from the page" } });
  fireEvent.click(screen.getByRole("button", { name: "Push and create" }));
  await waitFor(() => expect(gh.openPull).toHaveBeenCalledWith("Stop from the page", ""));
});

it("starts from the agent's draft and asks for a new one with Generate", () => {
  const gh = github();
  render(
    <CreatePullRequest
      pull={pull({ openAt: "https://example.test/compare", draft: { title: "Drafted", body: "Why." } })}
      branch="feature/x"
      agent="Claude Code"
      github={gh}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Create pull request…" }));
  expect((screen.getByLabelText("Title") as HTMLInputElement).value).toBe("Drafted");
  fireEvent.click(screen.getByRole("button", { name: "Generate" }));
  expect(gh.generatePullDraft).toHaveBeenCalled();
});

it("waits for a new draft after Generate instead of taking the one already there", async () => {
  const gh = github();
  const old = pull({ openAt: "x", draft: { title: "Old", body: "Old." } });
  const { rerender } = render(<CreatePullRequest pull={old} branch="feature/x" agent="Claude Code" github={gh} />);

  fireEvent.click(screen.getByRole("button", { name: "Create pull request…" }));
  fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Mine" } });
  fireEvent.click(screen.getByRole("button", { name: "Generate" }));
  await waitFor(() => expect(screen.getByRole("button", { name: /Asked Claude Code/ })).toBeTruthy());
  expect((screen.getByLabelText("Title") as HTMLInputElement).value, "the old draft is not the answer").toBe("Mine");

  rerender(
    <CreatePullRequest
      pull={pull({ openAt: "x", draft: { title: "New", body: "New." } })}
      branch="feature/x"
      agent="Claude Code"
      github={gh}
    />,
  );
  await waitFor(() => expect((screen.getByLabelText("Title") as HTMLInputElement).value).toBe("New"));
});
