import { useCallback, useEffect, useRef, useState } from "react";
import { api, type CommentsView, type CommentView, type NewAbout } from "@/api";
import type { GitHubState } from "@/hooks/useGitHub";
import { usePending, type PendingActions } from "@/hooks/usePending";
import { useThreadFolds, type ThreadFolds } from "@/hooks/useThreadFolds";
import { onNudge } from "@/lib/watch";
import { said } from "@/lib/utils";

/** What the reviewer wrote back, and keeping it current.
 *
 * Fetched apart from the review: comments are the one thing on the page the
 * reader writes, and reloading the whole map after every sentence would move
 * the blocks under someone who is mid-file. Reloading after a write rather than
 * patching the list in place is the same choice `useReview` makes for the tick
 * boxes — the server is what decides, here as there. */
export function useComments(
  onError: (message: string) => void,
  generation = 0,
  github: GitHubState | null = null,
): CommentActions {
  const { write, ...threads } = useThreads(onError, generation, github);
  return { ...threads, ...useThreadFolds(), ...usePending(github, write) };
}

/** The threads themselves: fetching them, keeping them current, and writing
 * them, each write followed by asking again for everything it can touch. */
export function useThreads(
  onError: (message: string) => void,
  generation: number,
  github: GitHubState | null,
) {
  const [found, setFound] = useState<CommentsView>(EMPTY);
  const [loaded, setLoaded] = useState(false);
  const latest = useRef(0);

  const load = useCallback(async () => {
    const request = ++latest.current;
    try {
      const next = await api.comments();
      if (request === latest.current) {
        setFound(next);
        setLoaded(true);
      }
    } catch (e) {
      if (request === latest.current) onError(said(e));
    }
  }, [onError]);

  useEffect(() => {
    setFound(EMPTY);
    setLoaded(false);
    void load();
    return () => { latest.current++; };
  }, [load, generation]);

  // Applied, not announced. A comment file edited in an editor next door is the
  // reader's own writing coming back, so it lands quietly — unlike a map that
  // moved, which asks before it redraws the page.
  useEffect(() => onNudge("comments", () => void load()), [load]);

  const reloadGitHub = github?.reload;
  // Everything a write can touch is asked again after it, the pull request
  // included: a pending comment changes on GitHub without its thread's file
  // changing at all.
  const write = useCallback(
    async (job: Promise<unknown>) => {
      try {
        await job;
      } catch (e) {
        onError(said(e));
        await reloadGitHub?.();
        return false;
      }
      await Promise.all([load(), reloadGitHub?.()]);
      return true;
    },
    [load, onError, reloadGitHub],
  );

  return {
    /** This file's comments: what is counted beside it and what is drawn under
     * its lines, which are the same array because the server sent them as one.
     * A lookup and not a filter — the cut was decided before it got here. */
    on: useCallback((path: string) => found.files[path] ?? NONE, [found.files]),
    /** A lens's threads, by the key its section carries. */
    onLens: useCallback((key: string) => found.lenses[key] ?? NONE, [found.lenses]),
    /** The threads on a block's description. */
    onBlock: useCallback((slug: string) => found.blocks[slug] ?? NONE, [found.blocks]),
    unreadable: found.unreadable,
    /** Whether the threads have come back at least once. */
    loaded,
    /** Every thread, on the diff or off it, for what counts them across files. */
    all: [
      ...Object.values(found.files).flat(),
      ...Object.values(found.lenses).flat(),
      ...Object.values(found.blocks).flat(),
      ...found.drifted,
    ],
    /** Threads whose lines the diff no longer shows. */
    drifted: found.drifted,
    add: (about: NewAbout, body: string) => write(api.addComment(about, body)),
    close: (id: string) => write(api.closeComment(id)),
    /** Write back under a comment, and with `andSend` hand the conversation on. */
    reply: (id: string, body: string, andSend: boolean) =>
      write(
        api.replyComment(id, body).then((comment) =>
          andSend ? api.sendComment(comment.id) : undefined,
        ),
      ),
    /** Write it and hand it to the agent in one go. A send that fails leaves
     * the comment saved, so nothing typed is lost and Send can be pressed again. */
    addAndSend: (about: NewAbout, body: string) =>
      write(api.addComment(about, body).then((comment) => api.sendComment(comment.id))),
    /** Which harness a Send reaches, or null when no session is connected. */
    agent: found.agent,
    /** True once the harness took it. Nothing to reload: sending changes no
     * comment, and the one the agent closes comes back through the watcher. */
    send: async (id: string) => {
      try {
        await api.sendComment(id);
        return true;
      } catch (e) {
        onError(said(e));
        return false;
      }
    },
    /** For the pending comments, which reload the same way after a write. */
    write,
  };
}

export type ThreadActions = Omit<ReturnType<typeof useThreads>, "write">;

/** Everything the screen can do to a comment, handed down to the diff: the
 * threads, which of them are folded, and their comments for the pull request.
 * A component that needs only part of it takes a `Pick` of it. */
export type CommentActions = ThreadActions & ThreadFolds & PendingActions;

/** What one drawn thread can do: its question, its conversation, its comment
 * for the pull request. Every place a thread is drawn hands down this much
 * and no more. */
export type DrawnThreadActions = Pick<
  CommentActions,
  | "agent"
  | "pendingOffered"
  | "pendingOf"
  | "isMinimized"
  | "toggleMinimized"
  | "writePending"
  | "editPending"
  | "removePending"
  | "generate"
  | "send"
  | "reply"
  | "close"
>;

/** Threads on prose, a lens or a block's description, and starting one. */
export type ProseActions = DrawnThreadActions & Pick<CommentActions, "add" | "addAndSend">;

/** A lens: its threads, found by the lens. */
export type LensActions = ProseActions & Pick<CommentActions, "onLens">;

/** A block's description: its threads, found by the block. */
export type BlockActions = ProseActions & Pick<CommentActions, "onBlock">;

/** A file's diff: threads under their lines, starting one on a span, and the
 * pending comments no thread points at. */
export type DiffActions = DrawnThreadActions &
  Pick<CommentActions, "add" | "addAndSend" | "addAsPending" | "looseOn">;

/** A file's comments, asked for by path.
 *
 * Handed around as this rather than as the whole list plus a filter at each
 * stop: a consumer that cannot narrow cannot narrow differently. */
export type CommentsOn = (path: string) => CommentView[];

/** One array for every file without a comment, so a row that has none is not
 * handed a fresh one on each render. */
const NONE: CommentView[] = [];

const EMPTY: CommentsView = { files: {}, lenses: {}, blocks: {}, drifted: [], unreadable: [], agent: null };
