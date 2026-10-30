import { useEffect, useState } from "react";
import type { GitHubState } from "@/hooks/useGitHub";
import type { PullDraftView } from "@/api";

/** What the Create pull request form holds: the title and description being
 * written, whether the agent was asked for a draft, and whether the pull
 * request is being opened.
 *
 * The agent's draft lands while the form is open, and replaces what is in it
 * only when the reviewer asked for one: a draft arriving on its own never
 * overwrites what they typed. */
export function usePullRequestForm(draft: PullDraftView | null, github: GitHubState) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [asked, setAsked] = useState(false);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    if (!draft || !asked) return;
    setTitle(draft.title);
    setBody(draft.body);
    setAsked(false);
  }, [draft, asked]);

  return {
    title,
    body,
    setTitle,
    setBody,
    asked,
    creating,
    /** Start from the agent's draft when the form is still empty. */
    begin: () => {
      if (draft && !title && !body) {
        setTitle(draft.title);
        setBody(draft.body);
      }
    },
    generate: async () => setAsked(await github.generatePullDraft()),
    /** True once the pull request is open. */
    create: async () => {
      setCreating(true);
      const opened = await github.openPull(title, body);
      setCreating(false);
      return opened !== null;
    },
  };
}

export type PullRequestForm = ReturnType<typeof usePullRequestForm>;
