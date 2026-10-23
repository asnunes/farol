import { useState } from "react";
import { ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { span } from "@/review/diff/comment/quoted";
import type { CommentView, PullRequestView, SentView, Verdict } from "@/api";

/** Submitting the review: the verdict and the summary, over the pending
 * comments already on the pull request.
 *
 * The dialog is the confirmation — it lists exactly what goes, and says how
 * many threads stay behind, because threads never leave farol and the reader
 * should see that difference at the one moment it matters. */
export function SubmitReview({ pull, threads, onSubmit }: SubmitReviewProps) {
  const [open, setOpen] = useState(false);
  const [summary, setSummary] = useState("");
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState<SentView | null>(null);

  const going = threads.filter((thread) => pull.pending[thread.id]?.state === "pending");
  const staying = threads.length - going.length;

  async function submit(verdict: Verdict) {
    setBusy(true);
    const answer = await onSubmit(verdict, summary);
    setBusy(false);
    if (answer) {
      setSent(answer);
      setSummary("");
    }
  }

  return (
    <>
      <Button
        size="xs"
        className="submit-review bg-comment-ink text-surface hover:bg-comment-ink/90"
        onClick={() => {
          setSent(null);
          setOpen(true);
        }}
      >
        Submit review…
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="border-rule bg-surface sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="font-sans text-base text-ink">
              {sent
                ? "Review submitted"
                : `Submit review · ${going.length} pending comment${going.length === 1 ? "" : "s"}`}
            </DialogTitle>
            <DialogDescription className="text-ink-muted">
              {sent
                ? `${sent.comments} comment${sent.comments === 1 ? "" : "s"} went with it.`
                : `On pull request #${pull.number}.`}
            </DialogDescription>
          </DialogHeader>

          {sent ? (
            <div className="flex flex-col gap-2 font-sans text-sm text-ink-soft">
              <a href={sent.url} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 underline">
                View it on GitHub <ExternalLink className="size-3.5" aria-hidden="true" />
              </a>
              {sent.readFailed ? (
                <p className="text-ink-muted">The files you had read were not ticked: {sent.readFailed}</p>
              ) : sent.read > 0 ? (
                <p className="text-ink-muted">{sent.read} file(s) ticked as read on the pull request.</p>
              ) : null}
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              <ul className="flex max-h-56 flex-col gap-1.5 overflow-y-auto font-sans text-sm">
                {going.map((thread) => (
                  <li key={thread.id} className="text-ink-soft">
                    <span className="font-mono text-xs text-comment-ink">
                      {thread.path.split("/").pop()}:{span(thread, "–")}
                    </span>{" "}
                    — {excerpt(pull.pending[thread.id]?.body ?? "")}
                  </li>
                ))}
              </ul>
              {staying > 0 && (
                <p className="font-sans text-xs text-ink-muted">
                  {staying} thread{staying === 1 ? " stays" : "s stay"} in farol and {staying === 1 ? "is" : "are"} not sent.
                </p>
              )}
              <Textarea
                aria-label="Summary of the review"
                className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
                rows={3}
                placeholder="Summary (optional, needed to request changes)"
                value={summary}
                disabled={busy}
                onChange={(e) => setSummary(e.target.value)}
              />
              {pull.mine && (
                <p className="font-sans text-xs text-ink-muted">
                  You cannot approve or request changes on your own pull request.
                </p>
              )}
              <div className="flex flex-wrap justify-end gap-2">
                <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setOpen(false)}>
                  Cancel
                </Button>
                <Button size="xs" variant="outline" disabled={busy} onClick={() => void submit("comment")}>
                  Comment
                </Button>
                {!pull.mine && (
                  <>
                    <Button
                      size="xs"
                      variant="outline"
                      disabled={busy || !summary.trim()}
                      title={summary.trim() ? undefined : "Say what to change in the summary first"}
                      onClick={() => void submit("request-changes")}
                    >
                      Request changes
                    </Button>
                    <Button
                      size="xs"
                      className="bg-comment-ink text-surface hover:bg-comment-ink/90"
                      disabled={busy}
                      onClick={() => void submit("approve")}
                    >
                      Approve
                    </Button>
                  </>
                )}
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}

/** The start of a comment, as a list row has room for. */
function excerpt(body: string): string {
  const line = body.trim().split("\n")[0] ?? "";
  return line.length > 80 ? `${line.slice(0, 80)}…` : line;
}

type SubmitReviewProps = {
  pull: PullRequestView;
  /** Every thread on the review, to tell what goes from what stays. */
  threads: CommentView[];
  onSubmit: (verdict: Verdict, summary: string) => Promise<SentView | null>;
};
