import { describe, expect, it } from "vitest";
import { block, file as fileView, review as reviewOf } from "./review/diff/testing";
import { blockAt, blockOf, lensAt, lensKeyOf, lensPlace, readingOrder, readingRows, type FileView, type ReviewView } from "./api";

function file(path: string, viewed = false): FileView {
  return fileView({ path, viewed });
}

function review(): ReviewView {
  return reviewOf([
    block("one", [file("a.rs", true), file("b.rs")], { title: "First" }),
    block("two", [file("c.rs")], { title: "Second" }),
  ]);
}

describe("reading order", () => {
  it("walks blocks in order", () => {
    // The order the keyboard moves through has to match the order on screen,
    // or j/k would jump around.
    expect(readingOrder(review()).map((f) => f.path)).toEqual([
      "a.rs",
      "b.rs",
      "c.rs",
    ]);
  });

  it("finds the next unread from wherever you are", () => {
    const order = readingOrder(review());
    const next = order.slice(1).find((f) => !f.viewed);
    expect(next?.path).toBe("b.rs");
  });

  it("announces each block above the files it holds", () => {
    // What the pane draws. It used to assemble this itself, and the comment on
    // the test above — that the keys and the screen have to agree — was a hope
    // rather than something the code held to.
    expect(
      readingRows(review()).map((row) => ("file" in row ? row.file.path : `[${row.block.slug}]`)),
    ).toEqual(["[one]", "a.rs", "b.rs", "[two]", "c.rs"]);
  });

  it("walks the keys through exactly the files the page draws", () => {
    const drawn = readingRows(review()).flatMap((row) => ("file" in row ? [row.file.path] : []));

    expect(readingOrder(review()).map((f) => f.path)).toEqual(drawn);
  });

  it("numbers the blocks in the order they are met", () => {
    const bands = readingRows(review()).flatMap((row) => ("block" in row ? [row.number] : []));

    expect(bands).toEqual([1, 2]);
  });
});

describe("block lookup", () => {
  it("reports which block a file is read under, and its position", () => {
    const r = review();
    expect(blockOf(r, "c.rs")).toMatchObject({ index: 1 });
    expect(blockOf(r, "c.rs")?.block.title).toBe("Second");
  });

  it("returns nothing for a file no block reads", () => {
    expect(blockOf(review(), "Cargo.lock")).toBeNull();
  });
});

describe("the block the reader is in", () => {
  it("is the block of the file they are on, or of the lens they are on", () => {
    // The `[` `]` keys asked only about files, so on a block's lens they did
    // nothing while the sidebar showed the block as current.
    const r = review();
    r.blocks[1].lenses = [{ id: "summary", key: "block:two:summary" } as ReviewView["lenses"][number]];

    expect(blockAt(r, "c.rs")?.index).toBe(1);
    expect(blockAt(r, lensPlace("block:two:summary"))?.index).toBe(1);
    expect(blockAt(r, null)).toBeNull();
  });
});

describe("the lens the reader is on", () => {
  it("is named as a place and read back from it, and a file path names no lens", () => {
    expect(lensKeyOf(lensPlace("block:two:architecture"))).toBe("block:two:architecture");
    expect(lensKeyOf("src/a.rs")).toBeNull();
  });

  it("is found by its key, with the block it opens, and a file path is never taken for one", () => {
    const r = review();
    const lens = {
      id: "architecture",
      key: "block:two:architecture",
      title: "Architecture",
    } as ReviewView["lenses"][number];
    r.blocks[1].lenses = [lens];
    r.lenses = [{ ...lens, id: "overview", key: "pr:overview" }];

    expect(lensAt(r, "lens:block:two:architecture")).toEqual({ lens, block: r.blocks[1] });
    expect(lensAt(r, "lens:pr:overview")?.block).toBeNull();
    expect(lensAt(r, "c.rs"), "a file is a file, even beside a lens").toBeNull();
  });
});
