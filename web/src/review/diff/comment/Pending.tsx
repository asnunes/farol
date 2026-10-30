import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Prose } from "@/review/Prose";
import { Generate } from "./Generate";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView, RemotePendingView } from "@/api";

/** The thread's comment for the pull request, as GitHub holds it.
 *
 * A card of its own under the thread, as the approved mockup draws it: the
 * state is a small pill with a dot — amber while it waits, green once it went
 * — and nothing else is coloured, so the card reads as the thread's outcome
 * rather than as the point of the thread.
 *
 * Drawn the same with no thread above it, for a comment written on the pull
 * request's page or left by a thread that was closed: then it goes by its own
 * id, and there is no conversation to ask the agent from. */
export function Pending({ id, thread, remote, actions, busy }: PendingProps) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState("");
  const by = thread?.pending?.name ?? "you";

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
          onClick={() => void actions.removePending(id)}
        >
          Forget it
        </Button>
      </p>
    );
  }

  const published = remote.state === "published";

  return (
    <div className="pending overflow-hidden rounded-lg border border-rule-strong bg-surface">
      <div className="flex items-center gap-2 border-b border-rule px-2.5 py-1.5 font-sans text-xs text-ink-muted">
        <span className="inline-flex items-center gap-1.5 rounded-full border border-rule-strong px-2 text-[0.6875rem] font-medium text-ink-soft">
          <span
            className={published ? "size-1.5 rounded-full bg-add-ink" : "size-1.5 rounded-full bg-highlight"}
            aria-hidden="true"
          />
          {published ? "Published" : "Pending"}
        </span>
        {published && remote.url ? (
          <a href={remote.url} target="_blank" rel="noreferrer" className="text-add-ink hover:underline">
            view on GitHub ↗
          </a>
        ) : (
          <span>for the pull request</span>
        )}
        <span className="ml-auto font-mono">written by {by}</span>
      </div>

      {editing ? (
        <div className="flex flex-col gap-2 px-3 py-2">
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
        <Prose className="body px-3 py-2 font-sans text-sm leading-relaxed text-ink">{remote.body ?? ""}</Prose>
      )}

      {/* A submitted comment is the pull request's now: nothing here can
          change it, so nothing here offers to. */}
      {!published && !editing && !busy && (
        <div className="flex flex-wrap items-center gap-1.5 px-2.5 pb-2">
          <Button
            variant="outline"
            size="xs"
            className="h-6 px-2 font-sans"
            onClick={() => {
              setText(remote.body ?? "");
              setEditing(true);
            }}
          >
            Edit
          </Button>
          {/* What the agent wrote can be written again; what the reviewer
              wrote is theirs, and the agent is asked about it instead. */}
          {actions.agent &&
            thread &&
            (thread.pending?.by === "agent" ? (
              <Generate label="Regenerate" onGenerate={(note) => actions.generate(thread.id, note)} />
            ) : (
              <Button
                variant="outline"
                size="xs"
                className="h-6 px-2 font-sans"
                onClick={() => void actions.send(thread.id)}
              >
                Ask {actions.agent} about it
              </Button>
            ))}
          <Button
            variant="ghost"
            size="xs"
            className="h-6 px-2 font-sans text-ink-muted"
            onClick={() => void actions.removePending(id)}
          >
            Remove
          </Button>
        </div>
      )}
    </div>
  );

  async function save() {
    if (!text.trim()) return;
    if (await actions.editPending(id, text)) setEditing(false);
  }
}

type PendingProps = {
  /** What edits and removals go by: the thread's id, or the comment's own. */
  id: string;
  /** The thread it came out of, when one still points at it. */
  thread: CommentView | null;
  /** Undefined until GitHub has answered. */
  remote: RemotePendingView | undefined;
  actions: CommentActions;
  /** An agent owes something on the thread, so nothing is offered meanwhile. */
  busy: boolean;
};
