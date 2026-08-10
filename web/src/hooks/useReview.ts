import { useCallback, useEffect, useState } from "react";
import { api, readingOrder, type ReviewView } from "@/api";

/** The map, and keeping it current.
 *
 * The repository moving is pushed, not polled: commit in another terminal and
 * the screen catches up on its own. */
export function useReview() {
  const [review, setReview] = useState<ReviewView | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const next = await api.review();
      setReview(next);
      setCurrent((prev) => {
        // Stay where the reader is, unless the file they were on is gone.
        if (prev && readingOrder(next).some((f) => f.path === prev)) return prev;
        const first = readingOrder(next).find((f) => !f.viewed) ?? readingOrder(next)[0];
        return first?.path ?? null;
      });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const es = new EventSource("/api/watch");
    const refresh = () => void load();
    es.addEventListener("map", refresh);
    es.addEventListener("head", refresh);
    return () => es.close();
  }, [load]);

  const toggleViewed = useCallback(
    async (path: string, viewed: boolean) => {
      await api.setViewed(path, viewed);
      await load();
    },
    [load],
  );

  return { review, current, setCurrent, error, setError, toggleViewed };
}
