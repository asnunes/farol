import { useCallback, useEffect, useRef, useState } from "react";
import { api, type CommentsView, type CommentView, type Side } from "@/api";
import { onNudge } from "@/lib/watch";
import { said } from "@/lib/utils";

/** What the reviewer wrote back, and keeping it current.
 *
 * Fetched apart from the review: comments are the one thing on the page the
 * reader writes, and reloading the whole map after every sentence would move
 * the blocks under someone who is mid-file. Reloading after a write rather than
 * patching the list in place is the same choice `useReview` makes for the tick
 * boxes — the server is what decides, here as there. */
export function useComments(onError: (message: string) => void, generation = 0) {
  const [found, setFound] = useState<CommentsView>({ files: {}, unreadable: [] });
  const latest = useRef(0);

  const load = useCallback(async () => {
    const request = ++latest.current;
    try {
      const next = await api.comments();
      if (request === latest.current) setFound(next);
    } catch (e) {
      if (request === latest.current) onError(said(e));
    }
  }, [onError]);

  useEffect(() => {
    setFound({ files: {}, unreadable: [] });
    void load();
    return () => { latest.current++; };
  }, [load, generation]);

  // Applied, not announced. A comment file edited in an editor next door is the
  // reader's own writing coming back, so it lands quietly — unlike a map that
  // moved, which asks before it redraws the page.
  useEffect(() => onNudge("comments", () => void load()), [load]);

  const write = useCallback(
    async (job: Promise<unknown>) => {
      try {
        await job;
      } catch (e) {
        onError(said(e));
        return;
      }
      await load();
    },
    [load, onError],
  );

  return {
    /** This file's comments: what is counted beside it and what is drawn under
     * its lines, which are the same array because the server sent them as one.
     * A lookup and not a filter — the cut was decided before it got here. */
    on: useCallback((path: string) => found.files[path] ?? NONE, [found.files]),
    unreadable: found.unreadable,
    add: (path: string, side: Side, from: number, to: number, body: string) =>
      write(api.addComment(path, side, from, to, body)),
    close: (id: string) => write(api.closeComment(id)),
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
