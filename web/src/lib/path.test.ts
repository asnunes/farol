import { describe, expect, it } from "vitest";
import { fileLabels } from "./path";

describe("naming the files of a list", () => {
  it("leaves a name that nothing else shares on its own", () => {
    const label = fileLabels(["web/src/App.tsx", "src/map/mod.rs"]);

    expect(label("web/src/App.tsx")).toEqual({ name: "App.tsx", where: "" });
  });

  it("adds the directory that separates two files of the same name", () => {
    const label = fileLabels(["web/src/App.tsx", "src/map/mod.rs", "src/diff/mod.rs"]);

    expect(label("src/map/mod.rs")).toEqual({ name: "mod.rs", where: "map" });
    expect(label("src/diff/mod.rs")).toEqual({ name: "mod.rs", where: "diff" });
  });

  it("goes further up while the nearest directory is shared too", () => {
    // One segment says `domain` for both, which is the ambiguity it was meant
    // to settle rather than repeat.
    const label = fileLabels(["src/map/domain/mod.rs", "src/diff/domain/mod.rs"]);

    expect(label("src/map/domain/mod.rs").where).toBe("map/domain");
    expect(label("src/diff/domain/mod.rs").where).toBe("diff/domain");
  });

  it("keeps the shortest tail that works, not the deepest available", () => {
    const label = fileLabels(["a/b/c/mod.rs", "a/b/d/mod.rs"]);

    expect(label("a/b/c/mod.rs").where).toBe("c");
  });

  it("gives a file at the root nothing to show, and its homonym the directory", () => {
    const label = fileLabels(["mod.rs", "src/mod.rs"]);

    expect(label("mod.rs").where).toBe("");
    expect(label("src/mod.rs").where).toBe("src");
  });

  it("falls back to the bare name for a path the list never held", () => {
    const label = fileLabels(["src/a.rs"]);

    expect(label("elsewhere/b.rs")).toEqual({ name: "b.rs", where: "" });
  });
});
