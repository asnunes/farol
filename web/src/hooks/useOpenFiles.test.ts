import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useOpenFiles } from "./useOpenFiles";
import type { FileView } from "@/api";

describe("which files are open", () => {
  it("opens what has not been read and folds what has", () => {
    const { result } = renderHook(() => useOpenFiles());

    expect(result.current.isOpen(file("a.rs", false))).toBe(true);
    expect(result.current.isOpen(file("a.rs", true))).toBe(false);
  });

  it("keeps a file the reader folded folded", () => {
    const { result } = renderHook(() => useOpenFiles());
    const unread = file("a.rs", false);

    act(() => result.current.set(unread, false));

    expect(result.current.isOpen(unread)).toBe(false);
  });

  it("opens a folded file again once the change underneath takes its tick off", () => {
    // The reported bug. Read is read *of this content*: the file moved on, the
    // server dropped the mark, and a fold kept as a decision of its own left it
    // minimised with nothing saying it had been read.
    const { result } = renderHook(() => useOpenFiles());

    act(() => result.current.set(file("a.rs", true), false));

    expect(result.current.isOpen(file("a.rs", true))).toBe(false);
    expect(result.current.isOpen(file("a.rs", false))).toBe(true);
  });

  it("closes a file the reader opened once it is marked read", () => {
    // The same rule the other way round, so it is a rule and not a special
    // case: opening a read file is an argument with its tick, and marking it
    // again is the tick answering.
    const { result } = renderHook(() => useOpenFiles());

    act(() => result.current.set(file("a.rs", false), false));

    expect(result.current.isOpen(file("a.rs", true))).toBe(false);
  });

  it("keeps each file's answer to itself", () => {
    const { result } = renderHook(() => useOpenFiles());

    act(() => result.current.set(file("a.rs", false), false));

    expect(result.current.isOpen(file("b.rs", false))).toBe(true);
  });
});

function file(path: string, viewed: boolean): FileView {
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
