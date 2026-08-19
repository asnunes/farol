import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Publish } from "./Publish";
import { comment } from "../diff/testing";
import type { Publishing } from "@/hooks/usePublishing";
import type { ReadinessView, SentView, Verdict } from "@/api";

describe("the send button", () => {
  it("is disabled until the review can actually go", () => {
    render(<Publish {...props(at("noToken"))} />);

    expect(button().hasAttribute("disabled")).toBe(true);
    expect(screen.getByLabelText("Why this review cannot be sent yet")).toBeTruthy();
  });

  it("drops the (i) once there is nothing left to explain", () => {
    render(<Publish {...props(at("ready", { pullRequest: 12 }))} />);

    expect(button().hasAttribute("disabled")).toBe(false);
    expect(screen.queryByLabelText("Why this review cannot be sent yet")).toBeNull();
  });

  it("counts what has not gone yet, and only that", () => {
    // A published comment is still open, so it is still on the page — but it
    // is not going a second time, and the number is what is about to be sent.
    render(
      <Publish
        {...props(at("ready", { pullRequest: 12 }), [
          comment({ id: "1" }),
          comment({ id: "2", published: "https://example.test/r1" }),
        ])}
      />,
    );

    expect(button().textContent).toContain("1");
  });

  it("waits for the first answer with a spinner, not with a reason", () => {
    // The answer comes from GitHub, so there is a moment with no reason to
    // give. The button is drawn and cannot be pressed, and the (i) stays away
    // until there is something behind it.
    render(<Publish {...props(null)} />);

    expect(button().hasAttribute("disabled")).toBe(true);
    expect(screen.getByLabelText("Checking whether this review can be sent")).toBeTruthy();
    expect(screen.queryByLabelText("Why this review cannot be sent yet")).toBeNull();
  });
});

describe("the panel that says what is missing", () => {
  it("asks for the token, and sends the reader nowhere else", () => {
    // The permission is named, because that is what the token has to carry.
    // What must not appear is work on the branch: nothing is wrong with it.
    open(at("noToken"));

    expect(screen.getByPlaceholderText("github_pat_…")).toBeTruthy();
    expect(screen.queryByText(/git push/)).toBeNull();
    expect(screen.queryByText(/open a pull request for this branch/i)).toBeNull();
  });

  it("covers the push and the pull request together, because both are missing", () => {
    // A branch GitHub has never seen cannot have a pull request. Naming only
    // the push would send the reader back a second time.
    open(at("branchNotPushed", { branch: "feat/x" }));

    expect(screen.getByText("git push -u origin feat/x")).toBeTruthy();
    expect(screen.getByRole("dialog").textContent).toMatch(/open a pull request for it/i);
  });

  it("offers the pull request form once the branch is there", () => {
    open(at("noPullRequest", { openAt: "https://example.test/compare" }));

    const link = screen.getByText(/open a pull request for this branch/i).closest("a");
    expect(link?.getAttribute("href")).toBe("https://example.test/compare");
    expect(screen.queryByText(/git push/)).toBeNull();
  });

  it("does not offer a token box where a token is not the problem", () => {
    // The one thing the reader must not be sent off to fix when it is fine.
    open(at("branchNotPushed"));

    expect(screen.queryByPlaceholderText("github_pat_…")).toBeNull();
  });
});

describe("sending", () => {
  it("will not send a verdict that asks for something without saying what", async () => {
    const publishing = props(at("ready", { pullRequest: 12 }), [comment({ id: "1" })]);
    render(<Publish {...publishing} />);
    fireEvent.click(button());

    fireEvent.click(screen.getByText("Request changes"));

    await waitFor(() => expect(send().hasAttribute("disabled")).toBe(true));
    expect(publishing.publishing.publish).not.toHaveBeenCalled();
  });

  it("sends an approval with nothing written, because approving needs no words", async () => {
    const publishing = props(at("ready", { pullRequest: 12 }), [comment({ id: "1" })]);
    render(<Publish {...publishing} />);
    fireEvent.click(button());
    fireEvent.click(screen.getByText("Approve"));

    await waitFor(() => expect(send().hasAttribute("disabled")).toBe(false));
    await act(async () => void fireEvent.click(send()));

    expect(publishing.publishing.publish).toHaveBeenCalledWith("approve", "");
    expect(screen.getByText("https://example.test/r1")).toBeTruthy();
  });

  it("offers only approve when there is nothing to send", () => {
    // Comment and request changes are somebody being asked for something, and
    // an empty review is not asking for anything.
    render(<Publish {...props(at("ready", { pullRequest: 12 }), [])} />);
    fireEvent.click(button());

    expect(screen.getByText("Approve")).toBeTruthy();
    expect(screen.queryByText("Request changes")).toBeNull();
  });
});

function button(): HTMLElement {
  return screen.getByText("Send review").closest("button")!;
}

function send(): HTMLElement {
  return screen.getByText("Send").closest("button")!;
}

function open(readiness: ReadinessView) {
  render(<Publish {...props(readiness)} />);
  fireEvent.click(screen.getByLabelText("Why this review cannot be sent yet"));
}

function at(
  state: ReadinessView["state"],
  over: Partial<ReadinessView> = {},
): ReadinessView {
  return { state, branch: "feature/x", ...over };
}

function props(readiness: ReadinessView | null, comments = [comment({ id: "1" })]) {
  const publishing: Publishing = {
    readiness,
    ask: vi.fn().mockResolvedValue(undefined),
    publish: vi
      .fn<(verdict: Verdict, summary: string) => Promise<SentView>>()
      .mockResolvedValue({ url: "https://example.test/r1", comments: 1 }),
    saveToken: vi.fn().mockResolvedValue(undefined),
  };
  return { publishing, comments, onError: vi.fn() };
}
