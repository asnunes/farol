import { useCallback, useEffect, useMemo, useState } from "react";
import type { CommentView, PullRequestView, WaitingView } from "@/api";
import { waitingLine } from "@/review/diff/comment/waiting";
import { span } from "@/review/diff/comment/quoted";

/** What the agents did on the threads, for the box in the corner.
 *
 * Derived from what the page already holds rather than sent by the server: an
 * answer is a turn in a thread, a pending comment is a thread's pending one,
 * and work in progress is a thread's wait. The watcher already brings each of
 * those in as it happens, so the box only has to notice the difference.
 *
 * What the reader has seen is kept in the browser, per branch, so a reload
 * does not turn everything new again. The first time a branch is opened,
 * everything already there counts as seen: it was there before the box was,
 * and listing it as new would bury the one answer that is. */
export function useActivity(
  threads: CommentView[],
  pull: PullRequestView | null,
  branch: string,
  loaded: boolean,
) {
  const key = `farol:activity:${branch}`;
  const [memory, setMemory] = useState<Memory>(() => load(key));
  // Another branch has its own memory.
  useEffect(() => setMemory(load(key)), [key]);

  const events = useMemo(() => happened(threads, pull), [threads, pull]);

  // A new event is stamped with when the page first saw it — which is when it
  // happened, give or take a nudge, for the ones the server has no clock for.
  useEffect(() => {
    // Not before the threads have arrived: an empty first answer would mark
    // the branch as seen, and everything already on it would then turn up new.
    if (!loaded) return;
    const now = Math.floor(Date.now() / 1000);
    const unknown = events.filter((e) => memory.firstSeen[e.key] === undefined);
    if (unknown.length === 0 && memory.started) return;
    const next: Memory = {
      started: true,
      firstSeen: { ...memory.firstSeen },
      read: memory.started ? memory.read : [...memory.read, ...unknown.map((e) => e.key)],
    };
    for (const e of unknown) next.firstSeen[e.key] = e.at ?? now;
    setMemory(next);
    save(key, next);
  }, [events, memory, key, loaded]);

  const markRead = useCallback(
    (keys: string[]) =>
      setMemory((was) => {
        const next = { ...was, read: [...new Set([...was.read, ...keys])] };
        save(key, next);
        return next;
      }),
    [key],
  );

  const updates = events
    .map((e) => ({ ...e, at: e.at ?? memory.firstSeen[e.key] ?? 0, read: memory.read.includes(e.key) }))
    .sort((a, b) => b.at - a.at)
    .slice(0, LIMIT);

  return {
    working: working(threads),
    updates,
    fresh: updates.filter((u) => !u.read).length,
    read: (item: Update) => markRead([item.key]),
    readAll: () => markRead(updates.map((u) => u.key)),
  };
}

/** How many updates the box keeps; older ones are on the threads themselves. */
const LIMIT = 30;

export type Activity = ReturnType<typeof useActivity>;

/** Something an agent finished: an answer, or a pending comment. */
export type Update = {
  key: string;
  thread: CommentView;
  who: string;
  what: string;
  excerpt: string;
  /** Seconds since the epoch. */
  at: number;
  read: boolean;
};

/** A thread an agent owes something on right now: handed over, or started. */
export type Working = {
  thread: CommentView;
  waiting: WaitingView;
  /** The line the row reads as, the same words the thread's own badge uses. */
  line: string;
  excerpt: string;
  since: number;
};

/** Where a thread sits, short enough for a row: `module.go:26`. */
export function where(thread: CommentView): string {
  return `${thread.path.split("/").pop()}:${span(thread, "–")}`;
}

/** "just now", "2 min ago", "3 h ago". */
export function ago(seconds: number, now = Date.now() / 1000): string {
  const gone = Math.max(0, Math.floor(now - seconds));
  if (gone < 45) return "just now";
  if (gone < 3600) return `${Math.round(gone / 60)} min ago`;
  if (gone < 86400) return `${Math.round(gone / 3600)} h ago`;
  return `${Math.round(gone / 86400)} d ago`;
}

function happened(threads: CommentView[], pull: PullRequestView | null): Event[] {
  const out: Event[] = [];
  for (const thread of threads) {
    thread.replies.forEach((reply, i) => {
      if (reply.from !== "agent") return;
      out.push({
        key: `${thread.id}:reply:${i}`,
        thread,
        who: reply.name,
        what: "answered",
        excerpt: firstLine(reply.body),
        at: Number(reply.at) || undefined,
      });
    });
    if (thread.pending?.by === "agent") {
      out.push({
        key: `${thread.id}:pending:${thread.pending.id}`,
        thread,
        who: thread.pending.name,
        what: "wrote a pending comment",
        excerpt: firstLine(pull?.pending[thread.id]?.body ?? ""),
        at: undefined,
      });
    }
  }
  return out;
}

/** Every thread an agent owes something on, from the moment it was sent.
 *
 * Not only from the acknowledgement: between the send and the agent saying it
 * started, the thread is already waiting, and a box that stayed quiet then
 * looked as though the send had not gone. */
function working(threads: CommentView[]): Working[] {
  return threads.flatMap((thread) => {
    const waiting = thread.waiting;
    if (!waiting) return [];
    return [
      {
        thread,
        waiting,
        line: waitingLine(waiting),
        excerpt: firstLine(thread.body),
        since: Number(waiting.since) || 0,
      },
    ];
  });
}

/** The start of what was said, without the markup that would show as noise. */
function firstLine(text: string): string {
  return (text.trim().split("\n")[0] ?? "").replace(/[`*_>#]/g, "").trim();
}

function load(key: string): Memory {
  try {
    const raw = window.localStorage?.getItem(key);
    if (raw) return JSON.parse(raw) as Memory;
  } catch {
    // A storage the browser will not open is a box that forgets on reload,
    // which is better than one that fails to draw.
  }
  return { started: false, read: [], firstSeen: {} };
}

function save(key: string, memory: Memory) {
  try {
    window.localStorage?.setItem(key, JSON.stringify(memory));
  } catch {
    // As above.
  }
}

type Event = Omit<Update, "at" | "read"> & { at: number | undefined };

/** What the browser keeps about the box, per branch. */
type Memory = {
  /** Whether the branch has been looked at before, which decides whether what
   * is already there is new. */
  started: boolean;
  read: string[];
  /** When the page first saw each event, for the ones with no clock of their own. */
  firstSeen: Record<string, number>;
};
