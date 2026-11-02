import { api, type LoosePendingView, type NewAbout, type RemotePendingView } from "@/api";
import type { GitHubState } from "@/hooks/useGitHub";

/** The comments for the pull request: whether threads can have one here, each
 * thread's as GitHub holds it, and writing, changing and taking them back.
 *
 * Apart from the threads because it changes with the pull request and the
 * review mode, which the threads know nothing about. */
export function usePending(github: GitHubState | null, write: Write) {
  const pull = github?.pull ?? null;

  return {
    /** Whether threads can have comments for the pull request here: a peer
     * review, with a pull request farol can write on. */
    pendingOffered: pull?.mode.mode === "peer" && pull.connection === "ready",
    /** The thread's pending comment as GitHub has it, when it has one and
     * GitHub has answered. */
    pendingOf: (id: string): RemotePendingView | undefined => pull?.pending[id],
    /** A file's pending comments that no thread points at. */
    looseOn: (path: string): LoosePendingView[] => pull?.loose.filter((one) => one.path === path) ?? [],
    writePending: (id: string, body: string) => write(api.writePending(id, body)),
    editPending: (id: string, body: string) => write(api.editPending(id, body)),
    removePending: (id: string) => write(api.removePending(id)),
    generate: (id: string, instruction = "") => write(api.generatePending(id, instruction)),
    /** A thread whose first words go to the pull request as they are. */
    addAsPending: (about: NewAbout, body: string) =>
      write(api.addComment(about, body).then((comment) => api.writePending(comment.id, body))),
  };
}

export type PendingActions = ReturnType<typeof usePending>;

/** Run a write, then ask again for everything it can touch. */
type Write = (job: Promise<unknown>) => Promise<boolean>;
