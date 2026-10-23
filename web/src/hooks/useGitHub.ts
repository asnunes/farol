import { useCallback, useEffect, useRef, useState } from "react";
import { api, type PullRequestView, type ReviewMode, type SentView, type Verdict } from "@/api";
import { onNudge } from "@/lib/watch";
import { said } from "@/lib/utils";

/** The pull request, the review mode, and each thread's pending comment as
 * GitHub holds it.
 *
 * Asked apart from the threads, because this one goes to the network and the
 * threads should be on screen before it answers. Asked again whenever the
 * threads change: a pending comment the agent wrote from its terminal arrives
 * as a change to its thread's file, and its words are on GitHub. */
export function useGitHub(onError: (message: string) => void, generation = 0) {
  const [pull, setPull] = useState<PullRequestView | null>(null);
  const latest = useRef(0);

  const load = useCallback(async () => {
    const request = ++latest.current;
    try {
      const next = await api.github();
      if (request === latest.current) setPull(next);
    } catch (e) {
      if (request === latest.current) onError(said(e));
    }
  }, [onError]);

  useEffect(() => {
    void load();
    return () => { latest.current++; };
  }, [load, generation]);

  useEffect(() => onNudge("comments", () => void load()), [load]);
  useEffect(() => onNudge("head", () => void load()), [load]);

  return {
    /** Null until GitHub answers the first time. */
    pull,
    reload: load,
    chooseMode: async (mode: ReviewMode) => {
      try {
        await api.chooseMode(mode);
      } catch (e) {
        onError(said(e));
      }
      await load();
    },
    /** What the submitted review came to, or null when it did not go. */
    submit: async (verdict: Verdict, summary: string): Promise<SentView | null> => {
      try {
        const sent = await api.submitReview(verdict, summary);
        await load();
        return sent;
      } catch (e) {
        onError(said(e));
        return null;
      }
    },
  };
}

export type GitHubState = ReturnType<typeof useGitHub>;
