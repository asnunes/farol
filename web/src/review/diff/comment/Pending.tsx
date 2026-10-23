import { useState } from "react";
import { ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Prose } from "@/review/Prose";
import { Generate } from "./Generate";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView, RemotePendingView } from "@/api";

/** The thread's comment for the pull request, as GitHub holds it.
 *
 * Quiet on purpose: a dot and a label, not a coloured box. The thread is what
 * the reader is working in, and this is what will leave it; marking it loudly
 * would make the comment look like the point of the thread rather than its
 * outcome. */
export function Pending({ comment, remote, actions, busy }: PendingProps) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState("");
  const by = comment.pending?.name ?? "you";

  if (remote === undefined) {
    return (
      <p className="pending unknown font-sans text-xs text-ink-muted">
        Pending comment · not read from GitHub yet
      </p>
    );
  }

  if (remote.state === "gone") {
    return (
      <p className="pending gone flex items-center gap-1.5 font-sans text-xs text-ink-muted">
        The pending comment is no longer on the pull request.
        <Button
          variant="link"
          size="xs"
          className="h-auto p-0 text-xs text-comment-ink underline"
          onClick={() => void actions.removePending(comment.id)}
        >
          Forget it
        </Button>
      </p>
    );
  }

  const published = remote.state === "published";

  return (
    <div className="pending flex flex-col gap-1.5 border-l-2 border-rule-strong pl-3">
      <span className="flex items-center gap-1.5 font-mono text-xs text-ink-muted">
        <span
          className={published ? "size-1.5 rounded-full bg-add-ink" : "size-1.5 rounded-full bg-highlight"}
          aria-hidden="true"
        />
        <span className="font-semibold text-ink-soft">{published ? "Published" : "Pending"}</span>
        {published && remote.url ? (
          <a href={remote.url} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 underline">
            view on GitHub <ExternalLink className="size-3" aria-hidden="true" />
          </a>
        ) : (
          <span>for the pull request</span>
        )}
        <span>· written by {by}</span>
      </span>

      {editing ? (
        <div className="flex flex-col gap-2">
          <Textarea
            autoFocus
            aria-label="Edit the pending comment"
            className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
            rows={3}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                void save();
              }
              if (e.key === "Escape") {
                e.preventDefault();
                setEditing(false);
              }
            }}
          />
          <div className="flex gap-2">
            <Button
              size="xs"
              className="bg-comment-ink text-surface hover:bg-comment-ink/90"
              disabled={!text.trim()}
              onClick={() => void save()}
            >
              Save
            </Button>
            <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setEditing(false)}>
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <Prose className="body font-sans text-sm leading-relaxed text-ink-soft">{remote.body ?? ""}</Prose>
      )}

      {/* A submitted comment is the pull request's now: nothing here can
          change it, so nothing here offers to. */}
      {!published && !editing && !busy && (
        <div className="flex flex-wrap items-center gap-1">
          <Button
            variant="ghost"
            size="xs"
            className="text-comment-faint hover:bg-sunken hover:text-comment-ink"
            onClick={() => {
              setText(remote.body ?? "");
              setEditing(true);
            }}
          >
            Edit
          </Button>
          {actions.agent && (
            <Generate label="Regenerate" onGenerate={(note) => actions.generate(comment.id, note)} />
          )}
          <Button
            variant="ghost"
            size="xs"
            className="text-comment-faint hover:bg-sunken hover:text-comment-ink"
            onClick={() => void actions.removePending(comment.id)}
          >
            Remove
          </Button>
        </div>
      )}
    </div>
  );

  async function save() {
    if (!text.trim()) return;
    if (await actions.editPending(comment.id, text)) setEditing(false);
  }
}

type PendingProps = {
  comment: CommentView;
  /** Undefined until GitHub has answered. */
  remote: RemotePendingView | undefined;
  actions: CommentActions;
  /** An agent owes something on the thread, so nothing is offered meanwhile. */
  busy: boolean;
};
