import { afterEach, describe, expect, it } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { ago, useActivity } from "./useActivity";
import { comment } from "@/review/diff/testing";
import type { CommentView } from "@/api";

describe("agent activity", () => {
  afterEach(() => window.localStorage?.clear());

  it("counts nothing as new on a branch opened for the first time", () => {
    // It was all there before the box was; listing it as new would bury the
    // one answer that is.
    const { result } = renderHook(() => useActivity([answered("t1")], null, "b1", true));

    expect(result.current.updates).toHaveLength(1);
    expect(result.current.fresh).toBe(0);
  });

  it("counts an answer that arrives afterwards as new, until it is read", () => {
    const { result, rerender } = renderHook(
      ({ threads }) => useActivity(threads, null, "b2", true),
      { initialProps: { threads: [comment({ id: "t1" })] } },
    );

    rerender({ threads: [answered("t1")] });
    expect(result.current.fresh).toBe(1);
    expect(result.current.updates[0].what).toBe("answered");

    act(() => result.current.read(result.current.updates[0]));
    expect(result.current.fresh).toBe(0);
  });

  it("remembers what was read across a reload", () => {
    const first = renderHook(({ threads }) => useActivity(threads, null, "b3", true), {
      initialProps: { threads: [comment({ id: "t1" })] },
    });
    first.rerender({ threads: [answered("t1")] });
    act(() => first.result.current.readAll());
    first.unmount();

    const again = renderHook(() => useActivity([answered("t1")], null, "b3", true));

    expect(again.result.current.fresh).toBe(0);
  });

  it("does not take the empty list before the first load as the branch's first look", () => {
    // Taken as the first look, it would make every answer already on the
    // branch look new once the threads arrive.
    const { result, rerender } = renderHook(
      ({ threads, loaded }) => useActivity(threads, null, "b4", loaded),
      { initialProps: { threads: [] as CommentView[], loaded: false } },
    );

    rerender({ threads: [answered("t1")], loaded: true });

    expect(result.current.fresh).toBe(0);
  });

  it("lists an agent at work, and a pending comment it wrote, by what it is doing", () => {
    const writing = comment({
      id: "t1",
      waiting: { on: "Codex", stage: "working", purpose: "pending", since: "100" },
    });
    const wrote = comment({ id: "t2", pending: { id: "PRRC_2", by: "agent", name: "Codex" } });

    const { result } = renderHook(() => useActivity([writing, wrote], null, "b5", true));

    expect(result.current.working[0].line).toBe("Codex is writing a pending comment");
    expect(result.current.updates[0].what).toBe("wrote a pending comment");
  });

  it("counts a regenerated pending comment as new again", () => {
    const { result, rerender } = renderHook(
      ({ threads }) => useActivity(threads, null, "b6", true),
      {
        initialProps: {
          threads: [comment({ id: "t1", pending: { id: "PRRC_1", by: "agent", name: "Codex" } })],
        },
      },
    );

    rerender({
      threads: [comment({ id: "t1", pending: { id: "PRRC_2", by: "agent", name: "Codex" } })],
    });

    expect(result.current.fresh).toBe(1);
  });

  it("says how long ago in the words a person uses", () => {
    expect(ago(1000, 1010)).toBe("just now");
    expect(ago(1000, 1000 + 120)).toBe("2 min ago");
    expect(ago(1000, 1000 + 3 * 3600)).toBe("3 h ago");
  });

  it("lists a thread as waiting from the moment it is sent, before the agent starts", () => {
    const sent = comment({
      id: "t1",
      waiting: { on: "Claude Code", stage: "sent", purpose: "answer", since: "100" },
    });

    const { result } = renderHook(() => useActivity([sent], null, "b7", true));

    expect(result.current.working[0].line).toBe("Sent to Claude Code · waiting for an answer");
    expect(result.current.working[0].started).toBe(false);
  });
});

function answered(id: string): CommentView {
  return comment({
    id,
    replies: [{ from: "agent", name: "Claude Code", body: "Because.", at: "200" }],
  });
}
