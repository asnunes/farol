import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { SentView, Verdict } from "@/api";

/** The summary and the verdict, written at the end and sent together.
 *
 * The summary is written here rather than accumulated as the reader goes,
 * because it is the one part of a review that can only be written once the
 * whole of it has been read. */
export function Send({ open, onOpenChange, waiting, pullRequest, publish, onError }: SendProps) {
  // With nothing to send, approving is the only verdict on offer: a review
  // that asks for something has to have said what, and there is nothing here
  // that says it. A clean approval is also the commonest one there is.
  const alone = waiting === 0;
  const [verdict, setVerdict] = useState<Verdict>(alone ? "approve" : "comment");
  const [summary, setSummary] = useState("");
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState<SentView | null>(null);

  // GitHub's rule, and a fair one: approving needs no words, and the other two
  // are somebody being asked to do something, where "why" is the whole of it.
  const needsSummary = verdict !== "approve";
  const short = needsSummary && !summary.trim();

  async function send() {
    if (short || sending) return;
    setSending(true);
    try {
      setSent(await publish(verdict, summary));
    } catch (e) {
      onError(String(e));
      onOpenChange(false);
    } finally {
      setSending(false);
    }
  }

  function close(next: boolean) {
    onOpenChange(next);
    if (!next) {
      setSummary("");
      setSent(null);
    }
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="send-card border-rule bg-surface sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="font-sans text-base text-ink">
            {sent ? "Review sent" : "Send review"}
          </DialogTitle>
          <DialogDescription className="font-serif text-ink-soft">
            {sent ? (
              <>
                {sent.comments === 0
                  ? "Your verdict is on the pull request."
                  : `${sent.comments} comment${sent.comments === 1 ? "" : "s"} went with it.`}{" "}
                They stay here too, still open, until you close them.
              </>
            ) : (
              <>
                {waiting === 0
                  ? "No comments are waiting to go."
                  : `${waiting} comment${waiting === 1 ? "" : "s"} will go with it.`}{" "}
                {pullRequest !== undefined && `Pull request #${pullRequest}.`}
              </>
            )}
          </DialogDescription>
        </DialogHeader>

        {sent ? (
          <a
            className="font-mono text-sm break-all text-accent underline"
            href={sent.url}
            target="_blank"
            rel="noreferrer"
          >
            {sent.url}
          </a>
        ) : (
          <>
            <Textarea
              className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
              rows={5}
              placeholder={
                needsSummary
                  ? "What the change does well, and what it still needs. Markdown."
                  : "Anything to add. Optional when approving."
              }
              value={summary}
              disabled={sending}
              onChange={(e) => setSummary(e.target.value)}
            />

            <ToggleGroup
              type="single"
              className="verdicts justify-start gap-2"
              value={verdict}
              onValueChange={(next) => next && setVerdict(next as Verdict)}
            >
              {(alone ? APPROVE_ONLY : ALL).map(([value, label, why]) => (
                <ToggleGroupItem
                  key={value}
                  value={value}
                  title={why}
                  className="cursor-pointer rounded border border-rule px-3 font-sans text-xs text-ink-soft data-[state=on]:bg-accent-dim data-[state=on]:text-accent"
                >
                  {label}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </>
        )}

        <DialogFooter>
          {sent ? (
            <Button size="sm" className="cursor-pointer" onClick={() => close(false)}>
              Done
            </Button>
          ) : (
            <>
              <Button
                size="sm"
                variant="ghost"
                className="cursor-pointer text-muted"
                onClick={() => close(false)}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                className="cursor-pointer"
                disabled={short || sending}
                title={
                  short ? "A review that asks for something has to say what" : undefined
                }
                onClick={() => void send()}
              >
                {sending ? "Sending…" : "Send"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** The three the API takes, in the order a reviewer works through them. */
const ALL: [Verdict, string, string][] = [
  ["comment", "Comment", "Leave the comments without a verdict"],
  ["requestChanges", "Request changes", "Ask for the change to be reworked"],
  ["approve", "Approve", "Say it is good to merge"],
];

const APPROVE_ONLY = ALL.filter(([value]) => value === "approve");

type SendProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Comments that have not gone yet — what is about to be sent. */
  waiting: number;
  pullRequest?: number;
  publish: (verdict: Verdict, summary: string) => Promise<SentView>;
  onError: (message: string) => void;
};
