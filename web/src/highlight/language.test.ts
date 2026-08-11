import { describe, expect, it } from "vitest";
import { languageOf } from "./language";

/** Stands in for the grammars Shiki ships. */
const shipped = (...ids: string[]) => (id: string) => ids.includes(id);

describe("naming the language of a file", () => {
  it("translates the extensions whose name is not the language", () => {
    expect(languageOf("src/server/mod.rs", shipped("rust"))).toBe("rust");
    expect(languageOf("web/src/api.ts", shipped("typescript"))).toBe("typescript");
    expect(languageOf("scripts/build.sh", shipped("shellscript"))).toBe("shellscript");
  });

  it("lets an extension that is already a language name through untranslated", () => {
    // The table would otherwise have to list `go`, `css`, `json`, `lua` and a
    // long tail besides, which is a second place to forget one.
    expect(languageOf("cmd/main.go", shipped("go"))).toBe("go");
    expect(languageOf("web/src/styles.css", shipped("css"))).toBe("css");
    expect(languageOf("Cargo.toml", shipped("toml"))).toBe("toml");
  });

  it("reads the files whose whole name is the clue", () => {
    expect(languageOf("Dockerfile", shipped("dockerfile"))).toBe("dockerfile");
    expect(languageOf("deploy/Makefile", shipped("make"))).toBe("make");
  });

  it("does not care how the name was capitalised", () => {
    expect(languageOf("build/DOCKERFILE", shipped("dockerfile"))).toBe("dockerfile");
    expect(languageOf("src/Main.RS", shipped("rust"))).toBe("rust");
  });

  it("answers nothing when there is nothing to answer", () => {
    // A normal answer, not a failure: the file is drawn plain, which is how
    // every file was drawn before any of this existed.
    expect(languageOf("LICENSE", shipped("rust"))).toBeNull();
    expect(languageOf(".gitkeep", shipped("rust"))).toBeNull();
    expect(languageOf("notes.wat", shipped("rust"))).toBeNull();
  });

  it("refuses a language it named but cannot load", () => {
    // The table can outlive a grammar; claiming one that is not there would
    // ask Shiki for something it does not have.
    expect(languageOf("src/mod.rs", shipped())).toBeNull();
  });
});
