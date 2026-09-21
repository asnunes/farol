import { describe, expect, it } from "vitest";
import { buttonVariants } from "./button";

/** One assertion, over one deviation from what shadcn ships.
 *
 * Its ghost variant carries `dark:hover:bg-accent/50` alongside
 * `hover:bg-accent`. The two have different modifiers, so tailwind-merge does
 * not see them as the same class: a caller writing `hover:bg-sunken` replaces
 * the first and the second survives, and in the dark the `dark:` one wins.
 * Fourteen buttons in this app set their own hover background and every one of
 * them had it quietly taken away — including the armed close button, whose text
 * is dark because its background is meant to be light, which left it at 1.1:1
 * and invisible. */
describe("the ghost button", () => {
  it("leaves the hover background to whoever uses it", () => {
    expect(buttonVariants({ variant: "ghost" })).not.toMatch(/dark:hover:bg-/);
  });

  it("still has a hover of its own for the buttons that ask for none", () => {
    expect(buttonVariants({ variant: "ghost" })).toMatch(/(?<!dark:)hover:bg-accent\b/);
  });
});
