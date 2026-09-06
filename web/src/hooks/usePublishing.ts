import { useCallback, useEffect, useRef, useState } from "react";
import { api, type ReadinessView, type SentView, type Verdict } from "@/api";

/** Whether the review can be sent, kept current without being asked for.
 *
 * The answer changes outside the page — a token pasted into a file, a branch
 * pushed from a terminal, a pull request opened in another tab — and every one
 * of those happens while the reader is somewhere else. So the question is asked
 * again when they come back, and only while the answer is still no: once it is
 * yes, nothing about coming back to the tab could make it no again, and asking
 * would be a request to GitHub per tab switch for the rest of the session.
 *
 * In an ordinary session this is one question at load and never again. */
export function usePublishing() {
  const [readiness, setReadiness] = useState<ReadinessView | null>(null);
  const asking = useRef(false);

  const ask = useCallback(async () => {
    if (asking.current) return;
    asking.current = true;
    try {
      setReadiness(await api.readiness());
    } catch {
      // Left as it was. A question that could not be asked is not an answer of
      // no, and turning a hiccup into "you have no token" would send the reader
      // off to fix something that is not broken.
    } finally {
      asking.current = false;
    }
  }, []);

  useEffect(() => {
    void ask();
  }, [ask]);

  const ready = readiness?.state === "ready";
  useEffect(() => {
    if (ready) return;
    // Returning to a tab fires both of these, and a window manager can fire
    // focus several times over. Waiting a moment before asking collapses the
    // burst into one question — and unlike a floor measured from the last
    // answer, it never swallows a reader who genuinely went and came back.
    let queued: number | undefined;
    const again = () => {
      if (document.visibilityState !== "visible") return;
      window.clearTimeout(queued);
      queued = window.setTimeout(() => void ask(), SETTLE);
    };
    document.addEventListener("visibilitychange", again);
    window.addEventListener("focus", again);
    return () => {
      window.clearTimeout(queued);
      document.removeEventListener("visibilitychange", again);
      window.removeEventListener("focus", again);
    };
  }, [ask, ready]);

  /** The ticks with no review in front of them. */
  const ticks = useCallback(async () => (await api.ticks()).read, []);

  const publish = useCallback(
    async (verdict: Verdict, summary: string): Promise<SentView> => {
      try {
        return await api.publish(verdict, summary);
      } catch (e) {
        // The refusal may be the state having moved under us — a revoked
        // token, a pull request that went on without us. Ask again so the
        // panel explains it rather than the reader pressing send twice.
        void ask();
        throw e;
      }
    },
    [ask],
  );

  const saveToken = useCallback(
    async (host: string, token: string) => {
      await api.saveToken(host, token);
      await ask();
    },
    [ask],
  );

  return { readiness, ask, publish, ticks, saveToken };
}

/** Everything the top bar can do about publishing. */
export type Publishing = ReturnType<typeof usePublishing>;

/** How long the events of one return to the tab take to stop arriving. */
const SETTLE = 150;
