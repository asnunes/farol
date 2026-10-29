import { useState } from "react";
import { MessageSquarePlus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Box } from "@/review/diff/comment/Box";
import { Comment } from "@/review/diff/comment/Comment";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView, NewAbout } from "@/api";

/** The threads on a lens or on a block's description, and the way to start
 * one: a button under the text, opening the same box a line of code opens.
 *
 * No comment for the pull request here: GitHub takes one only on a line of
 * the diff, and this text is not on it. */
export function ProseThreads({ about, label, threads, actions }: ProseThreadsProps) {
  const [writing, setWriting] = useState(false);
  const here: CommentActions = { ...actions, pendingOffered: false };

  return (
    <div className="prose-threads mt-2">
      {threads.map((thread) => (
        <Comment key={thread.id} comment={thread} actions={here} />
      ))}
      {!about ? null : writing ? (
        <Box
          label={label}
          agent={actions.agent}
          onSave={async (body) => {
            if (await actions.add(about, body)) setWriting(false);
          }}
          onSaveAndSend={async (body) => {
            if (await actions.addAndSend(about, body)) setWriting(false);
          }}
          onCancel={() => setWriting(false)}
        />
      ) : (
        <Button
          variant="ghost"
          size="xs"
          className="addthread -ml-2 gap-1 font-sans text-xs text-ink-muted hover:bg-transparent hover:text-comment-ink"
          onClick={() => setWriting(true)}
        >
          <MessageSquarePlus className="size-3.5" aria-hidden="true" />
          Comment
        </Button>
      )}
    </div>
  );
}

type ProseThreadsProps = {
  /** What a new thread will be about, or null where none can be started —
   * a lens folded away or not written yet still shows the threads it has. */
  about: NewAbout | null;
  /** The box's name: "Thread on the Overview lens", "Thread on this block". */
  label: string;
  threads: CommentView[];
  actions: CommentActions;
};
