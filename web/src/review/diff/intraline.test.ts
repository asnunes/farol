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

  it("marks a line inside a run of several, when the run is line for line", () => {
    // Three lines out and three in, each one the line above it edited. The
    // pairing is by position and here that is exactly right.
    const changed = changedRanges(
      '    es.addEventListener("map", refresh);',
      '    es.addEventListener("map", announce);',
    )!;

    expect(lit('    es.addEventListener("map", refresh);', changed.before)).toEqual(["refresh"]);
    expect(lit('    es.addEventListener("map", announce);', changed.after)).toEqual(["announce"]);
  });

  it("does not count the margin as something the two lines have in common", () => {
    // Two lines at the same depth share their indentation whatever they say.
    // Measured with it, this pair looked related enough to mark, and the marks
    // landed on a line nobody had edited.
    expect(
      changedRanges(
        "        assert_eq!(review.execute(&map).unwrap().commits_behind, 0);",
        "        editor.edit(|_| Ok::<_, MapError>(())).unwrap();",
      ),
    ).toBeNull();
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
