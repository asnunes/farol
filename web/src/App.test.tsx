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

/** The comment list, empty.
 *
 * Every stub answers this the same way: the tests here are about the map and
 * the diff, and without it a request for the comments falls through to the
 * branch that serves diffs, handing the page an object where it expects a list.
 * A test that is about comments stubs them itself. */
function emptyComments(url: string): Response | null {
  return url.startsWith("/api/comments")
    ? new Response('{"comments":[],"unreadable":[]}', {
        headers: { "content-type": "application/json" },
      })
    : null;
}

const emptyDiff = {
  path: "",
  status: "modified",
  hunks: [],
  binary: false,
  additions: 0,
  deletions: 0,
};

/** The file the page says the reader is on. Every file is on the page now, so
 * the first header in the document is not an answer to that question. */
function reading(): string {
  return document.querySelector(".pane")?.getAttribute("data-current") ?? "";
}

async function waitForReading(name: string) {
  await waitFor(() => expect(reading()).toContain(name));
}

/** Where navigation took the reader. Moving is scrolling now, so this is what
 * a key or a click actually does. */
const scrolls: string[] = [];

async function waitForScrollTo(name: string) {
  await waitFor(() => expect(scrolls.at(-1) ?? "").toContain(name));
}

/** One file's section, for the assertions that would otherwise match the same
 * thing once per file on the page. */
function section(path: string): HTMLElement {
  const found = document.querySelector<HTMLElement>(`[data-path="${path}"]`);
  if (!found) throw new Error(`no section for ${path}`);
  return found;
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
      const noComments = emptyComments(url);
      if (noComments) return noComments;
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
  // jsdom has neither of these, and the page is built on both: it scrolls to
  // move and watches sections to know where the reader is.
  scrolls.length = 0;
  Element.prototype.scrollIntoView = function (this: HTMLElement) {
    scrolls.push(this.dataset.path ?? "");
  };

  // Radix measures what it is about to place, and jsdom has no observer to
  // measure with. Nothing here asserts on size, so a stub that never fires is
  // enough to let a tooltip open.
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );

  // The page subscribes on mount; without a stub jsdom throws.
  vi.stubGlobal(
    "EventSource",
    class {
      addEventListener() {}
      removeEventListener() {}
      close() {}
    },
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("landing", () => {
  it("lays the whole review out in reading order, blocks and all", async () => {
    // One page rather than one file at a time, in the order the session chose.
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    const shown = [...document.querySelectorAll("[data-path]")].map(
      (el) => (el as HTMLElement).dataset.path,
    );
    expect(shown).toEqual(["src/a.rs", "src/b.rs", "src/c.rs"]);
    expect([...document.querySelectorAll(".blockbar h2")].map((h) => h.textContent)).toEqual([
      "The change itself",
      "The wiring",
    ]);
  });

  it("resumes at the first file that has not been read", async () => {
    // Every file is on the page, so resuming means being taken there rather
    // than being shown it alone.
    const r = review();
    r.blocks[0].files[0].viewed = true;
    r.viewedFiles = 1;
    serve({ review: r });

    render(<App />);
    await waitForScrollTo("b.rs");
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
    await waitForScrollTo("b.rs");

    // Third file lives in the second block: order is flat across blocks.
    fireEvent.keyDown(window, { key: "j" });
    await waitForScrollTo("c.rs");

    fireEvent.keyDown(window, { key: "k" });
    await waitForScrollTo("b.rs");
  });

  it("the arrows walk the files the way j and k do", async () => {
    // Both sets exist because the reader who knows the keys and the reader who
    // is guessing are the same person on different days.
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "ArrowDown" });
    await waitForScrollTo("b.rs");

    fireEvent.keyDown(window, { key: "ArrowUp" });
    await waitForScrollTo("a.rs");
  });

  it("n skips to the next file that has not been read", async () => {
    const r = review();
    r.blocks[0].files[1].viewed = true; // b.rs already read
    serve({ review: r });

    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "n" });
    await waitForScrollTo("c.rs");
  });

  it("brackets move a whole block at a time", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "]" });
    await waitForScrollTo("c.rs");

    fireEvent.keyDown(window, { key: "[" });
    await waitForScrollTo("a.rs");
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
  it("; sends the current file to the backend", async () => {
    const calls = serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: ";" });
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0]).toEqual({ path: "src/a.rs", viewed: true });
  });

  it("Space marks the file read, the way ; does", async () => {
    const calls = serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: " " });

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

    // Every file has a box now, so the click has to name which one.
    fireEvent.click(section("src/a.rs").querySelector(".markbox")!);
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
        const noComments = emptyComments(url);
        if (noComments) return noComments;
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

    // Every file on the page reports the same stubbed diff, so the claim is
    // about the one under test rather than about the page.
    await waitFor(() => expect(section("src/a.rs").textContent).toContain("Binary file"));
  });
});

describe("the diff itself", () => {
  // Every file on the page is served the same stubbed diff, so each claim here
  // is scoped to one section rather than counted across the whole page.
  /** A file whose diff has one changed line, with context around it. */
  function withDiff(diff: object, over: Partial<FileView> = {}) {
    const r = review();
    r.blocks[0].files[0] = file("src/a.rs", over);
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        const noComments = emptyComments(url);
        if (noComments) return noComments;
        if (url.startsWith("/api/review")) {
          return new Response(JSON.stringify(r), {
            headers: { "content-type": "application/json" },
          });
        }
        return new Response(JSON.stringify({ ...emptyDiff, path: "src/a.rs", ...diff }), {
          headers: { "content-type": "application/json" },
        });
      }),
    );
  }

  const oneChange = {
    hunks: [
      {
        old_start: 19,
        old_lines: 3,
        new_start: 19,
        new_lines: 3,
        lines: [
          { kind: "context", old_number: 19, new_number: 19, content: "line 19" },
          { kind: "removed", old_number: 20, new_number: null, content: "line 20" },
          { kind: "added", old_number: null, new_number: 20, content: "CHANGED" },
          { kind: "context", old_number: 21, new_number: 21, content: "line 21" },
        ],
      },
    ],
    additions: 1,
    deletions: 1,
  };

  it("marks added and removed lines apart from context", async () => {
    withDiff(oneChange);
    render(<App />);

    await waitFor(() => expect(section("src/a.rs").querySelectorAll(".row").length).toBe(4));
    expect(section("src/a.rs").querySelectorAll(".row.add").length).toBe(1);
    expect(section("src/a.rs").querySelectorAll(".row.del").length).toBe(1);
    expect(section("src/a.rs").querySelectorAll(".row:not(.add):not(.del)").length).toBe(2);
  });

  it("numbers a removed line by the side it still exists on", async () => {
    // An added line has no old number and a removed one has no new number;
    // showing a blank gutter would lose the reader's place.
    withDiff(oneChange);
    render(<App />);

    await waitFor(() => expect(section("src/a.rs").querySelectorAll(".row").length).toBe(4));
    const numbers = Array.from(section("src/a.rs").querySelectorAll(".row .ln")).map((n) => n.textContent);
    expect(numbers).toEqual(["19", "20", "20", "21"]);
  });

  it("shows the hunk header so the reader knows where they are in the file", async () => {
    withDiff(oneChange);
    render(<App />);

    await waitFor(() => expect(section("src/a.rs").textContent).toContain("@@ -19,3 +19,3 @@"));
  });

  it("puts a line note against the line it was written about", async () => {
    // The note belongs beside its code. Rendering it anywhere else is the same
    // as not having written it.
    withDiff(oneChange, {
      lineNotes: [
        { from: 20, to: 20, text: "this is the actual fix", block: "first" },
      ],
    });
    render(<App />);

    const note = await screen.findByText(/this is the actual fix/);
    const row = note.closest("div")?.previousElementSibling;
    expect(row?.textContent).toContain("CHANGED");
  });

  it("leaves a note off the lines it does not belong to", async () => {
    withDiff(oneChange, {
      lineNotes: [{ from: 99, to: 99, text: "about somewhere else", block: "first" }],
    });
    render(<App />);

    await waitFor(() => expect(section("src/a.rs").querySelectorAll(".row").length).toBe(4));
    expect(section("src/a.rs").querySelectorAll(".note").length).toBe(0);
  });
});

describe("open and closed", () => {
  it("folds a file that has been read, and leaves the header", async () => {
    // Read means done. The files still to read should not be buried under it.
    const r = review();
    r.blocks[0].files[0].viewed = true;
    r.viewedFiles = 1;
    serve({ review: r });

    render(<App />);
    await waitForReading("b.rs");

    expect(section("src/a.rs").textContent).toContain("a.rs");
    // Waited for, not asserted on the spot: the diff arrives from a fetch, and
    // the reading marker this test waited on says nothing about that.
    await waitFor(() => expect(section("src/b.rs").querySelector(".diff")).toBeTruthy());
    expect(section("src/a.rs").querySelector(".diff")).toBeNull();
  });

  it("opens a folded file when the reader asks for it", async () => {
    const r = review();
    r.blocks[0].files[0].viewed = true;
    r.viewedFiles = 1;
    serve({ review: r });

    render(<App />);
    await waitForReading("b.rs");

    fireEvent.click(section("src/a.rs").querySelector(".fold")!);

    await waitFor(() => expect(section("src/a.rs").querySelector(".diff")).toBeTruthy());
  });

  it("opens a folded file when it is picked in the sidebar", async () => {
    // Being taken to a file that stayed folded away would look like arriving
    // nowhere.
    const r = review();
    r.blocks[0].files[0].viewed = true;
    r.viewedFiles = 1;
    serve({ review: r });

    render(<App />);
    await waitForReading("b.rs");

    const target = Array.from(document.querySelectorAll(".fileitem")).find((b) =>
      b.textContent?.includes("a.rs"),
    );
    fireEvent.click(target!);

    await waitFor(() => expect(section("src/a.rs").querySelector(".diff")).toBeTruthy());
  });

  it("moves on to the next file after one is marked read", async () => {
    // Folding the file away leaves the reader looking at whatever was under it.
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: " " });

    await waitForScrollTo("b.rs");
  });

  it("folds a file the moment it is marked read", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: ";" });

    await waitFor(() => expect(section("src/a.rs").querySelector(".diff")).toBeNull());
  });
});

describe("light and dark", () => {
  it("puts the choice on the root element, which is what the stylesheet reads", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    const before = document.documentElement.dataset.theme;
    fireEvent.click(screen.getByTitle(/Switch to the/));

    await waitFor(() => expect(document.documentElement.dataset.theme).not.toBe(before));
    expect(localStorage.getItem("farol:theme")).toBe(document.documentElement.dataset.theme);
  });
});

describe("a long review", () => {
  it("asks for one diff per file, and only for the files on the page", async () => {
    // The pane is one page now. Fetching every diff up front would mean a
    // hundred requests before a branch of a hundred files showed anything.
    const asked: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        const noComments = emptyComments(url);
        if (noComments) return noComments;
        if (url.startsWith("/api/review")) {
          return new Response(JSON.stringify(review()), {
            headers: { "content-type": "application/json" },
          });
        }
        asked.push(new URL(url, "http://x").searchParams.get("path") ?? "");
        return new Response(JSON.stringify(emptyDiff), {
          headers: { "content-type": "application/json" },
        });
      }),
    );

    render(<App />);

    await waitFor(() => expect(asked).toHaveLength(3));
    expect(new Set(asked).size).toBe(3);
  });

  it("holds a big file back until it is asked for", async () => {
    // Thousands of changed lines cost everyone below them, and nobody scrolls
    // through a generated file. The header and the prose still show.
    const r = review();
    r.blocks[0].files[0] = file("src/a.rs", { additions: 900, deletions: 400 });
    const asked: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        const noComments = emptyComments(url);
        if (noComments) return noComments;
        if (url.startsWith("/api/review")) {
          return new Response(JSON.stringify(r), {
            headers: { "content-type": "application/json" },
          });
        }
        asked.push(new URL(url, "http://x").searchParams.get("path") ?? "");
        return new Response(JSON.stringify(emptyDiff), {
          headers: { "content-type": "application/json" },
        });
      }),
    );

    render(<App />);
    await waitFor(() => expect(asked).toContain("src/b.rs"));

    expect(asked).not.toContain("src/a.rs");
    expect(section("src/a.rs").textContent).toContain("1300 changed lines");

    fireEvent.click(section("src/a.rs").querySelector(".heavy button")!);

    await waitFor(() => expect(asked).toContain("src/a.rs"));
  });
});

describe("a comment file that cannot be read", () => {
  it("is said out loud, because the comment stops rendering either way", async () => {
    // Somebody opens the markdown and breaks the header. The comment vanishes
    // from the page, and silence about it reads as never having written it.
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.startsWith("/api/comments")) {
          return new Response(
            JSON.stringify({
              comments: [],
              unreadable: [
                {
                  file: ".git/farol/feature-x/comments/18cb-3731.md",
                  about: null,
                  excerpt: "Por que essa ordem?",
                  why: "its header is gone, so nothing says where it belongs",
                },
              ],
            }),
            { headers: { "content-type": "application/json" } },
          );
        }
        if (url.startsWith("/api/review")) {
          return new Response(JSON.stringify(review()), {
            headers: { "content-type": "application/json" },
          });
        }
        return new Response(JSON.stringify(emptyDiff), {
          headers: { "content-type": "application/json" },
        });
      }),
    );

    render(<App />);

    const chip = await waitFor(() => {
      const found = document.querySelector(".unreadable");
      expect(found).toBeTruthy();
      return found!;
    });
    expect(chip.textContent).toContain("1 comment unreadable");

    // The detail is a hover away. Known by what the reviewer wrote, since the
    // header no longer says which file it was about — and the file to open
    // comes with it, because that is the fix.
    fireEvent.pointerEnter(chip);
    fireEvent.focus(chip);
    const detail = await screen.findByRole("tooltip");
    expect(detail.textContent).toContain('"Por que essa ordem?"');
    expect(detail.textContent).toContain("header is gone");
    expect(detail.textContent).toContain("18cb-3731.md");
  });
});

describe("when the backend fails", () => {
  function serveBroken(failing: "review" | "file") {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        const noComments = emptyComments(url);
        if (noComments) return noComments;
        if (url.startsWith("/api/review")) {
          return failing === "review"
            ? new Response("the store is unreadable", { status: 400 })
            : new Response(JSON.stringify(review()), {
                headers: { "content-type": "application/json" },
              });
        }
        return new Response("'nowhere.rs' is not part of this review", { status: 400 });
      }),
    );
  }

  it("says why the review could not be loaded instead of showing an empty page", async () => {
    // An empty screen reads as "nothing to review", which is the opposite of
    // what happened.
    serveBroken("review");
    render(<App />);

    expect(await screen.findByText(/unreadable/)).toBeTruthy();
  });

  it("says why a file could not be loaded", async () => {
    serveBroken("file");
    render(<App />);

    expect(await screen.findByText(/not part of this review/)).toBeTruthy();
  });
});

describe("the skim list", () => {
  it("renders files that belong to no block at the bottom", async () => {
    // A lockfile has no story to sit in, but it is still in the diff and the
    // reviewer has to be able to reach it.
    const r = review();
    r.looseSkim = [file("Cargo.lock", { skim: true, skimReason: "regenerated" })];
    serve({ review: r });
    render(<App />);

    await waitForReading("a.rs");
    const loose = document.querySelectorAll(".blk-files");
    expect(
      Array.from(loose).some((ul) => ul.textContent?.includes("Cargo.lock")),
    ).toBe(true);
  });

  it("carries the reason as the tooltip so it can be read without leaving", async () => {
    const r = review();
    r.blocks[0].files[0] = file("src/a.rs", { skim: true, skimReason: "generated" });
    serve({ review: r });
    render(<App />);

    await waitFor(() => {
      const row = document.querySelector('[title="generated"]');
      expect(row).toBeTruthy();
    });
  });
});

describe("the help panel", () => {
  it("opens on ? and lists the keys", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "?" });

    expect(await screen.findByText("Keys")).toBeTruthy();
  });

  it("closes on escape, and stays open when the card itself is clicked", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");
    fireEvent.keyDown(window, { key: "?" });
    await screen.findByText("Keys");

    fireEvent.click(document.querySelector(".help-card")!);
    expect(screen.queryByText("Keys")).toBeTruthy();

    fireEvent.keyDown(document.querySelector(".help-card")!, { key: "Escape" });
    await waitFor(() => expect(screen.queryByText("Keys")).toBeNull());
  });

  it("is announced as a dialog rather than a floating box of text", async () => {
    // Screen readers get a named dialog and the focus is trapped inside it —
    // both of which the hand-rolled modal it replaced did not do.
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    fireEvent.keyDown(window, { key: "?" });

    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).toContain("Keys");
  });
});

describe("the sidebar", () => {
  it("moves the reader to the file that was clicked", async () => {
    // The keyboard is the fast path, but the sidebar is how you jump to a
    // file you spotted further down.
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    const target = Array.from(document.querySelectorAll(".fileitem")).find((b) =>
      b.textContent?.includes("c.rs"),
    );
    fireEvent.click(target!);

    await waitForScrollTo("c.rs");
    expect(reading()).toContain("c.rs");
  });

  it("marks the file being read so the reader can see where they are", async () => {
    serve({ review: review() });
    render(<App />);
    await waitForReading("a.rs");

    const marked = document.querySelector('.fileitem[aria-current="true"]');
    expect(marked?.textContent).toContain("a.rs");
  });
});
