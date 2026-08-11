import { describe, expect, it } from "vitest";
import { isBundled, load, tokenizerFor } from "./highlighter";

describe("loading a grammar", () => {
  it("hands out a tokenizer as soon as loading has finished", async () => {
    // The first render after a grammar arrives has to be the one that colours
    // the code. This used to need a second render — the highlighter was only
    // captured in a callback hung off its own promise — and on screen it read
    // as no colour at all until the reader opened another file and came back.
    await load("rust");

    expect(tokenizerFor("rust")).not.toBeNull();
  });

  it("colours the code it is given, line by line", async () => {
    await load("rust");
    const tokenize = tokenizerFor("rust")!;

    const lines = tokenize("// nota\npub fn x() {}");

    expect(lines).toHaveLength(2);
    expect(lines[0][0].content).toBe("// nota");
    // Both palettes ride on the token, which is what lets the stylesheet pick
    // one without anything being tokenized twice.
    expect(lines[0][0].style).toMatchObject({ "--shiki-light": expect.any(String) });
    expect(lines[0][0].style).toMatchObject({ "--shiki-dark": expect.any(String) });
  });

  it("says nothing for a language it was never asked to load", () => {
    expect(tokenizerFor("cobol")).toBeNull();
  });

  it("knows which languages it could load at all", () => {
    expect(isBundled("rust")).toBe(true);
    expect(isBundled("nao-existe")).toBe(false);
  });
});
