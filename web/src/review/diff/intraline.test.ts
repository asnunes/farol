import { describe, expect, it } from "vitest";
import { changedRanges } from "./intraline";

/** The marked pieces, as text, which is what a reader would see lit up. */
function lit(text: string, ranges: { from: number; to: number }[] | undefined) {
  return (ranges ?? []).map((r) => text.slice(r.from, r.to));
}

describe("marking what changed inside a line", () => {
  it("lights only the part that was added to the line", () => {
    const before = "export function TopBar({ review }: TopBarProps) {";
    const after = "export function TopBar({ review, view, onView }: TopBarProps) {";

    const changed = changedRanges(before, after)!;

    expect(lit(before, changed.before)).toEqual([]);
    expect(lit(after, changed.after)).toEqual([", view, onView"]);
  });

  it("lights the old word and the new one when a word is replaced", () => {
    const changed = changedRanges("const done = false;", "const done = true;")!;

    expect(lit("const done = false;", changed.before)).toEqual(["false"]);
    expect(lit("const done = true;", changed.after)).toEqual(["true"]);
  });

  it("joins neighbouring words into one mark", () => {
    // Five separate boxes for `, view, onView` would be harder to read than
    // the line without any marks at all.
    const changed = changedRanges("f(a)", "f(a, b, c)")!;

    expect(lit("f(a, b, c)", changed.after)).toEqual([", b, c"]);
  });

  it("says nothing when the two lines are a rewrite rather than an edit", () => {
    // Both lines would end up lit end to end, which says less than the row
    // colour already said.
    expect(changedRanges("const done = review.viewedFiles;", "await load(grammar);")).toBeNull();
  });

  it("says nothing when the line did not change at all", () => {
    const changed = changedRanges("const a = 1;", "const a = 1;")!;

    expect(changed.before).toEqual([]);
    expect(changed.after).toEqual([]);
  });

  it("compares words, not characters", () => {
    // Character by character, `review` and `viewed` share four letters and the
    // marks come out shredded.
    const changed = changedRanges("a review here", "a viewed here")!;

    expect(lit("a review here", changed.before)).toEqual(["review"]);
    expect(lit("a viewed here", changed.after)).toEqual(["viewed"]);
  });
});
