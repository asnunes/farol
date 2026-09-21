import { describe, expect, it } from "vitest";
import { blockOf, readingOrder, type FileView, type ReviewView } from "./api";

function file(path: string, viewed = false): FileView {
  return {
    path,
    status: "modified",
    additions: 1,
    deletions: 0,
    viewed,
    notes: [],
    lineNotes: [],
    tags: [],
    skim: false,
    skimReason: null,
  };
}

function review(): ReviewView {
  return {
    branch: "feature/x",
    base: "main",
    generatedAt: "abc1234",
    commitsBehind: 0,
    blocks: [
      { slug: "one", title: "First", context: "", files: [file("a.rs", true), file("b.rs")],
        totalFiles: 2, viewedFiles: 1 },
      { slug: "two", title: "Second", context: "", files: [file("c.rs")],
        totalFiles: 1, viewedFiles: 0 },
    ],
    looseSkim: [file("Cargo.lock")],
    unmapped: [],
    totalFiles: 4,
    viewedFiles: 1,
    firstUnread: "b.rs",
  };
}

describe("reading order", () => {
  it("walks blocks in order and puts loose skim last", () => {
    // The order the keyboard moves through has to match the order on screen,
    // or j/k would jump around.
    expect(readingOrder(review()).map((f) => f.path)).toEqual([
      "a.rs",
      "b.rs",
      "c.rs",
      "Cargo.lock",
    ]);
  });

  it("finds the next unread from wherever you are", () => {
    const order = readingOrder(review());
    const next = order.slice(1).find((f) => !f.viewed);
    expect(next?.path).toBe("b.rs");
  });
});

describe("block lookup", () => {
  it("reports which block a file is read under, and its position", () => {
    const r = review();
    expect(blockOf(r, "c.rs")).toMatchObject({ index: 1 });
    expect(blockOf(r, "c.rs")?.block.title).toBe("Second");
  });

  it("returns nothing for a file that only appears in loose skim", () => {
    // It still renders, it just has no block band above it.
    expect(blockOf(review(), "Cargo.lock")).toBeNull();
  });
});
