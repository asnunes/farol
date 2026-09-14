import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Sidebar } from "./Sidebar";
import type { BlockView, FileView, ReviewView } from "@/api";

describe("telling the files in the sidebar apart", () => {
  // Radix measures what it is about to place, and jsdom has no observer to
  // measure with. Nothing here asserts on size, so a stub that never fires is
  // enough to let a tooltip open.
  beforeAll(() => {
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
  });

  it("shows a unique name alone, with the path only in the tooltip", async () => {
    show(review([block("first", ["web/src/App.tsx"])]));

    const row = screen.getByRole("button", { name: "web/src/App.tsx" });
    expect(row.textContent).toContain("App.tsx");
    expect(row.textContent).not.toContain("web/src");

    fireEvent.focus(row);
    expect((await tooltip()).textContent).toContain("web/src/App.tsx");
  });

  it("separates homonyms that sit in different blocks", () => {
    // The map decides which block a file reads under, so two `mod.rs` are only
    // ever seen side by side in the sidebar — a per-block comparison would call
    // both of them unique.
    show(
      review([
        block("first", ["web/src/App.tsx", "src/map/mod.rs"]),
        block("second", ["src/diff/mod.rs"]),
      ]),
    );

    expect(named("src/map/mod.rs").textContent).toContain("map");
    expect(named("src/diff/mod.rs").textContent).toContain("diff");
    expect(named("web/src/App.tsx").textContent).not.toContain("web");
  });

  it("keeps adding segments until the directories differ", () => {
    show(
      review([
        block("first", ["src/map/domain/mod.rs"]),
        block("second", ["src/diff/domain/mod.rs"]),
      ]),
    );

    expect(named("src/map/domain/mod.rs").textContent).toContain("map/domain");
    expect(named("src/diff/domain/mod.rs").textContent).toContain("diff/domain");
  });

  it("keeps the reason a file can be skimmed, next to its path", async () => {
    show(
      review([], [file("src/gen/schema.rs", { skim: true, skimReason: "generated" })]),
    );

    fireEvent.focus(named("src/gen/schema.rs"));

    const text = (await tooltip()).textContent ?? "";
    expect(text).toContain("src/gen/schema.rs");
    expect(text).toContain("generated");
  });

  it("names the row by its whole path and opens the tooltip on keyboard focus", async () => {
    // A screen reader hears the path even where the eye sees only the name,
    // and reaching the row by Tab is enough to be told the rest.
    show(review([block("first", ["src/map/domain/mod.rs"])]));

    const row = screen.getByRole("button", { name: "src/map/domain/mod.rs" });
    expect(screen.queryByRole("tooltip")).toBeNull();

    row.focus();
    fireEvent.focus(row);

    expect(within(await tooltip()).getByText("src/map/domain/mod.rs")).toBeTruthy();
  });

  it("still picks the file it was clicked on", () => {
    const onPick = vi.fn();
    show(review([block("first", ["src/map/mod.rs"])]), onPick);

    fireEvent.click(named("src/map/mod.rs"));

    expect(onPick).toHaveBeenCalledWith("src/map/mod.rs");
  });
});

function show(view: ReviewView, onPick = vi.fn()) {
  render(
    <TooltipProvider>
      <Sidebar review={view} current={null} onPick={onPick} />
    </TooltipProvider>,
  );
}

/** The row for a path, found the way a screen reader would name it. */
function named(path: string): HTMLElement {
  return screen.getByRole("button", { name: path });
}

/** Radix renders the content twice while open — once on screen, once for
 * assistive tech — so the visible one is taken by role. */
function tooltip(): Promise<HTMLElement> {
  return screen.findByRole("tooltip");
}

function file(path: string, over: Partial<FileView> = {}): FileView {
  return {
    path,
    status: "modified",
    additions: 1,
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

function block(slug: string, paths: string[]): BlockView {
  return { slug, title: slug, context: "", files: paths.map((p) => file(p)) };
}

function review(blocks: BlockView[], looseSkim: FileView[] = []): ReviewView {
  return {
    branch: "feature/x",
    base: "main",
    generatedAt: "abc1234",
    commitsBehind: 0,
    blocks,
    looseSkim,
    unmapped: [],
    totalFiles: blocks.flatMap((b) => b.files).length + looseSkim.length,
    viewedFiles: 0,
  };
}
