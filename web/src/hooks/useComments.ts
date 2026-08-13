import { useCallback, useEffect, useState } from "react";
import { api, type CommentView } from "@/api";
import { onNudge } from "@/lib/watch";

/** What the reviewer wrote back, and keeping it current.
 *
 * Fetched apart from the review: comments are the one thing on the page the
 * reader writes, and reloading the whole map after every sentence would move
 * the blocks under someone who is mid-file. Reloading after a write rather than
 * patching the list in place is the same choice `useReview` makes for the tick
 * boxes — the server is what decides, here as there. */
export function useComments(onError: (message: string) => void) {
  const [comments, setComments] = useState<CommentView[]>([]);

  const load = useCallback(async () => {
    try {
      setComments(await api.comments());
    } catch (e) {
      onError(String(e));
    }
  }, [onError]);

  useEffect(() => {
    void load();
  }, [load]);

  // Applied, not announced. A comment file edited in an editor next door is the
  // reader's own writing coming back, so it lands quietly — unlike a map that
  // moved, which asks before it redraws the page.
  useEffect(() => onNudge("comments", () => void load()), [load]);

  const write = useCallback(
    async (job: Promise<unknown>) => {
      try {
        await job;
      } catch (e) {
        onError(String(e));
        return;
      }
      await load();
    },
    [load, onError],
  );

  return {
    comments,
    add: (path: string, from: number, to: number, body: string) =>
      write(api.addComment(path, from, to, body)),
    close: (id: string) => write(api.closeComment(id)),
  };
}

/** Everything the screen can do to a comment, handed down to the diff. */
export type CommentActions = ReturnType<typeof useComments>;
