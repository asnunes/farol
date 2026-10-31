import { act, renderHook } from "@testing-library/react";
import { expect, it } from "vitest";
import { useOpenLenses } from "./useOpenLenses";

it("a lens kept through its tick stays open once seen", () => {
  const { result } = renderHook(() => useOpenLenses());
  expect(result.current.isOpen("pr:risk", false)).toBe(true);

  act(() => result.current.keep("pr:risk", false));

  expect(result.current.isOpen("pr:risk", true)).toBe(true);
});

it("an untouched lens folds once seen", () => {
  const { result } = renderHook(() => useOpenLenses());
  expect(result.current.isOpen("pr:risk", true)).toBe(false);
});
