import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

/** The palette is a stylesheet, so the guard over it reads the stylesheet.
 *
 * Nothing on the page can be asked this question: jsdom resolves no
 * `light-dark()`, and a browser resolves only the half the machine is set to.
 * The two halves are written side by side in the sheet, which is where both can
 * be measured at once. */
const SHEET = readFileSync(`${import.meta.dirname}/styles.css`, "utf8");

/** Every surface farol's quiet ink is drawn on, which is what it has to hold
 * against. The last two are the ones that used to be forgotten: a selected file
 * row and the wash under the lines a comment is being picked on are backgrounds
 * as much as the page is. */
const UNDER = [
  "surface",
  "ground",
  "sunken",
  "add-bg",
  "del-bg",
  "note-bg",
  "comment-bg",
  "accent-dim",
  "comment-dim",
];

/** The surfaces an icon-only control sits on. Fewer, because an icon is never
 * put on a diff row or on a selected row. */
const BEHIND_ICONS = ["surface", "ground", "sunken", "comment-bg"];

describe.each(["light", "dark"] as const)("the %s palette", (theme) => {
  it.each(UNDER)("reads meaningful small text against %s", (bg) => {
    // `ink-muted` carries the directory that tells two `mod.rs` apart, the
    // skim marker, the line numbers and the band that says the file jumps.
    // None of it may be left to guesswork, which is 4.5:1 at this size.
    expect(ratio(token("muted", theme), token(bg, theme))).toBeGreaterThanOrEqual(4.5);
  });

  it.each(BEHIND_ICONS)("shows an icon-only control against %s", (bg) => {
    // A control, not prose: its meaning is in its label, and 3:1 is what the
    // glyph itself has to stand at. Held apart from the text ink on purpose —
    // brightening these to 4.5:1 would flatten the one distinction the two
    // tokens exist to draw.
    expect(ratio(token("faint", theme), token(bg, theme))).toBeGreaterThanOrEqual(3);
  });

  it("keeps the quiet inks in the order the design reads them in", () => {
    // Fixing the contrast by making everything the same colour would pass
    // every assertion above and lose the hierarchy they exist to protect.
    const on = token("surface", theme);
    expect(ratio(token("ink-soft", theme), on)).toBeGreaterThan(ratio(token("muted", theme), on));
    expect(ratio(token("muted", theme), on)).toBeGreaterThan(ratio(token("faint", theme), on));
  });
});

/** WCAG 2.1, 1.4.3 and 1.4.11: the same formula both thresholds are written
 * against. */
function ratio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((at) => {
    const part = parseInt(hex.slice(at, at + 2), 16) / 255;
    return part <= 0.04045 ? part / 12.92 : ((part + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

/** One half of a `light-dark()` pair, off the sheet. */
function token(name: string, theme: "light" | "dark"): string {
  const found = SHEET.match(
    new RegExp(`--farol-${name}: light-dark\\((#[0-9a-f]{6}), (#[0-9a-f]{6})\\)`),
  );
  if (!found) throw new Error(`no --farol-${name} in styles.css`);
  return theme === "light" ? found[1] : found[2];
}
