import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { FileView, ReviewView } from "./api";

function file(path: string, over: Partial<FileView> = {}): FileView {
  return {
    path,
    status: "modified",
    additions: 2,
    deletions: 0,
    viewed: false,
    notes: [],
    lineNotes: [],
    tags: [],
    skim: false,
    skimReason: null,
    ...over,
  };
}

function review(over: Partial<ReviewView> = {}): ReviewView {
  return {
    branch: "feature/x",
    base: "main",
    generatedAt: "abc1234",
    commitsBehind: 0,
    blocks: [
      {
        slug: "first",
        title: "The change itself",
        context: "why it exists",
        files: [file("src/a.rs"), file("src/b.rs")],
      },
      { slug: "second", title: "The wiring", context: "", files: [file("src/c.rs")] },
    ],
    looseSkim: [],
    unmapped: [],
    totalFiles: 3,
    viewedFiles: 0,
    ...over,
  };
}

const emptyDiff = {
  path: "",
  status: "modified",
  hunks: [],
  binary: false,
  additions: 0,
  deletions: 0,
};

/** The path in the file header — the file actually being read. A bare text
 * query would also match the sidebar entry, which is a different claim. */
function reading(): string {
  return document.querySelector(".filehead .path")?.textContent ?? "";
}

async function waitForReading(name: string) {
  await waitFor(() => expect(reading()).toContain(name));
}

/** The block band above the diff. The sidebar carries the same titles, so a
 * bare text query would not say which one is being read. */
function currentBlock(): string {
  return document.querySelector(".blockbar h2")?.textContent ?? "";
}

/** Stand in for the backend: the component only ever talks to it over fetch. */
function serve(state: { review: ReviewView }) {
  const viewedCalls: Array<{ path: string; viewed: boolean }> = [];

  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.startsWith("/api/review")) {
        return new Response(JSON.stringify(state.review), {
          headers: { "content-type": "application/json" },
        });
      }
      if (url.startsWith("/api/file")) {
        const path = new URL(url, "http://x").searchParams.get("path") ?? "";
        return new Response(JSON.stringify({ ...emptyDiff, path }), {
          headers: { "content-type": "application/json" },
        });
      }
      if (url.startsWith("/api/viewed")) {
        viewedCalls.push(JSON.parse(String(init?.body)));
        return new Response(null, { status: 204 });
      }
      throw new Error(`unexpected request to ${url}`);
    }),
  );

  return viewedCalls;
}

beforeEach(() => {
  // The page subscribes on mount; without a stub jsdom throws.
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      close() {}
    },
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("landing", () => {
  it("opens inside the first unread file, not on an index", async () => {
    serve({ review: review() });
    render(<App />);
    // The file header carries the path being read.
    await waitForReading("a.rs");
    expect(currentBlock()).toBe("The change itself");
  });

  it("resumes at the first file that has not been read", async () => {
    const r = review();
    r.blocks[0].files[0].viewed = true;
    r.viewedFiles = 1;
    serve({ review: r });

    render(<App />);
    await waitForReading("b.rs");
  });

  it("shows the success marker once everything is read", async () => {
    const r = review();
    r.blocks.forEach((b) => b.files.forEach((f) => (f.viewed = true)));
    r.viewedFiles = 3;
    serve({ review: r });

    render(<App />);
    await waitFor(() => expect(screen.getByTitle("Everything read")).toBeTruthy());
  });
});

describe("keyboard navigation", () => {
  it("j and k walk the reading order across block boundaries", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "j" });
    await waitForReading("b.rs");

    // Third file lives in the second block: order is flat across blocks.
    fireEvent.keyDown(window, { key: "j" });
    await waitForReading("c.rs");
    expect(currentBlock()).toBe("The wiring");

    fireEvent.keyDown(window, { key: "k" });
    await waitForReading("b.rs");
  });

  it("n skips to the next file that has not been read", async () => {
    const r = review();
    r.blocks[0].files[1].viewed = true; // b.rs already read
    serve({ review: r });

    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "n" });
    await waitForReading("c.rs");
  });

  it("brackets move a whole block at a time", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "]" });
    await waitForReading("c.rs");

    fireEvent.keyDown(window, { key: "[" });
    await waitForReading("a.rs");
  });

  it("? opens the shortcut list and Escape closes it", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "?" });
    expect(screen.getByText("Keys")).toBeTruthy();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByText("Keys")).toBeNull();
  });
});

describe("marking read", () => {
  it("e sends the current file to the backend", async () => {
    const calls = serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "e" });
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0]).toEqual({ path: "src/a.rs", viewed: true });
  });

  it("the checkbox toggles back off for a file already read", async () => {
    // Everything read, so the page lands on a file that is already ticked —
    // otherwise it would resume at the first unread one and the click would
    // be marking, not unmarking.
    const r = review();
    r.blocks.forEach((b) => b.files.forEach((f) => (f.viewed = true)));
    r.viewedFiles = 3;
    const calls = serve({ review: r });

    render(<App />);
    await waitForReading("a.rs");

    fireEvent.click(screen.getByTitle("Mark as read — key e"));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].viewed).toBe(false);
  });
});

describe("a file that belongs to two blocks", () => {
  it("is rendered once and carries both tags and both notes", async () => {
    // The server already resolved it to the earlier block; the page must show
    // what it was given without inventing a second appearance.
    serve({
      review: review({
        blocks: [
          {
            slug: "first",
            title: "First",
            context: "",
            files: [
              file("src/shared.rs", {
                tags: ["first", "second"],
                notes: [
                  { block: "first", text: "why it starts here" },
                  { block: "second", text: "why it matters again" },
                ],
              }),
            ],
          },
          { slug: "second", title: "Second", context: "", files: [] },
        ],
        totalFiles: 1,
      }),
    });

    render(<App />);
    await waitForReading("shared.rs");

    expect(reading()).toContain("shared.rs");
    expect(screen.getByText("why it starts here")).toBeTruthy();
    expect(screen.getByText("why it matters again")).toBeTruthy();
    const tags = [...document.querySelectorAll(".tags .tag")].map((e) => e.textContent);
    expect(tags).toEqual(["first", "second"]);
  });
});

describe("a map behind the branch", () => {
  it("says how far behind it is and lists what it does not cover", async () => {
    serve({
      review: review({ commitsBehind: 2, unmapped: ["src/arrived_later.rs"] }),
    });

    render(<App />);
    await waitFor(() => expect(screen.getByText(/2 commits behind/)).toBeTruthy());
    expect(screen.getByText("src/arrived_later.rs")).toBeTruthy();
  });
});

describe("a file with nothing to read", () => {
  it("says so instead of leaving the pane empty", async () => {
    // An empty diff area reads as a loading failure. A binary file has no
    // lines, and the reviewer needs to be told that rather than left guessing.
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.startsWith("/api/review")) {
          return new Response(JSON.stringify(review()), {
            headers: { "content-type": "application/json" },
          });
        }
        return new Response(JSON.stringify({ ...emptyDiff, path: "src/a.rs", binary: true }), {
          headers: { "content-type": "application/json" },
        });
      }),
    );

    render(<App />);

    expect(await screen.findByText(/Binary file/)).toBeTruthy();
  });
});
