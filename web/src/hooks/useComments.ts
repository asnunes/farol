import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  const [found, setFound] = useState<CommentsView>({ comments: [], unreadable: [] });
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
    setFound({ comments: [], unreadable: [] });
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

  // Grouped once, here, because every consumer wants the same cut of it: the
  // sidebar counts a file's questions, the header counts them again over the
  // code, and the diff draws them. Narrowed separately in each — which is what
  // this replaces — "how many comments does this file have" had four answers
  // and no way to notice when they stopped agreeing.
  const byPath = useMemo(() => {
    const grouped = new Map<string, CommentView[]>();
    for (const comment of found.comments) {
      const here = grouped.get(comment.path);
      if (here) here.push(comment);
      else grouped.set(comment.path, [comment]);
    }
    return grouped;
  }, [found.comments]);

  return {
    comments: found.comments,
    /** This file's comments: what is counted beside it and what is drawn under
     * its lines, which have to be the same list or the margin lies. */
    on: useCallback((path: string) => byPath.get(path) ?? NONE, [byPath]),
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
