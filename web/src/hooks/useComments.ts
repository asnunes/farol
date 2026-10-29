import { useCallback, useEffect, useRef, useState } from "react";
import { api, type CommentsView, type CommentView, type NewAbout, type RemotePendingView } from "@/api";
import type { GitHubState } from "@/hooks/useGitHub";
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
) {
  const [found, setFound] = useState<CommentsView>(EMPTY);
  const [loaded, setLoaded] = useState(false);
  // Threads folded to one line by the reader. Held here rather than in each
  // thread, because the activity box has to open one from the other end of
  // the page.
  const [minimized, setMinimized] = useState<ReadonlySet<string>>(new Set());
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

  const pull = github?.pull ?? null;

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
    isMinimized: (id: string) => minimized.has(id),
    toggleMinimized: (id: string) =>
      setMinimized((was) => {
        const next = new Set(was);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    expand: (id: string) =>
      setMinimized((was) => {
        if (!was.has(id)) return was;
        const next = new Set(was);
        next.delete(id);
        return next;
      }),
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
    close: (id: string, discardPending = false) => write(api.closeComment(id, discardPending)),
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
    /** Whether threads can have comments for the pull request here: a peer
     * review, with a pull request farol can write on. */
    pendingOffered: pull?.mode.mode === "peer" && pull.connection === "ready",
    /** The thread's pending comment as GitHub has it, when it has one and
     * GitHub has answered. */
    pendingOf: (id: string): RemotePendingView | undefined => pull?.pending[id],
    writePending: (id: string, body: string) => write(api.writePending(id, body)),
    editPending: (id: string, body: string) => write(api.editPending(id, body)),
    removePending: (id: string) => write(api.removePending(id)),
    generate: (id: string, instruction = "") => write(api.generatePending(id, instruction)),
    /** A thread whose first words go to the pull request as they are. */
    addAsPending: (about: NewAbout, body: string) =>
      write(api.addComment(about, body).then((comment) => api.writePending(comment.id, body))),
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
  };
}

/** Everything the screen can do to a comment, handed down to the diff. */
export type CommentActions = ReturnType<typeof useComments>;

/** A file's comments, asked for by path.
 *
 * Handed around as this rather than as the whole list plus a filter at each
 * stop: a consumer that cannot narrow cannot narrow differently. */
export type CommentsOn = (path: string) => CommentView[];

/** One array for every file without a comment, so a row that has none is not
 * handed a fresh one on each render. */
const NONE: CommentView[] = [];

const EMPTY: CommentsView = { files: {}, lenses: {}, blocks: {}, drifted: [], unreadable: [], agent: null };
