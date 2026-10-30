import { useCallback, useEffect, useRef, useState } from "react";
import { api, type OpenedPullView, type PullRequestView, type ReviewMode, type SentView, type Verdict } from "@/api";
import { onNudge } from "@/lib/watch";
import { usePageVisible } from "@/hooks/usePageVisible";
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
  // The agent wrote the pull request's draft: the open form fills itself.
  useEffect(() => onNudge("pull-request", () => void load()), [load]);

  // What is missing is fixed somewhere else — a pull request opened on
  // GitHub, `gh auth login` in a terminal — and nothing local announces it.
  // Coming back to the tab is when the reader expects it fixed, so that is
  // when the page asks again: once, and only while something is missing, so
  // a review that is ready does not reach GitHub on every switch of tabs.
  const visible = usePageVisible();
  const wasVisible = useRef(visible);
  const missing = pull !== null && pull.connection !== "ready" && pull.connection !== "no-remote";
  useEffect(() => {
    const returned = visible && !wasVisible.current;
    wasVisible.current = visible;
    if (returned && missing) void load();
  }, [visible, missing, load]);

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
    /** The pull request the host opened, or null when it did not open. */
    openPull: async (title: string, body: string): Promise<OpenedPullView | null> => {
      try {
        const opened = await api.openPull(title, body);
        await load();
        return opened;
      } catch (e) {
        onError(said(e));
        return null;
      }
    },
    /** Ask the agent for the draft; true once the harness took it. */
    generatePullDraft: async () => {
      try {
        await api.generatePullDraft();
        return true;
      } catch (e) {
        onError(said(e));
        return false;
      }
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
