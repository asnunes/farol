import { useState } from "react";
import {
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Verdicts } from "./Verdicts";
import type { SentView, Verdict } from "@/api";

/** Writing the review: what goes with it, the summary, and the verdict.
 *
 * The summary is written here at the end rather than accumulated as the reader
 * goes, because it is the one part of a review that can only be written once
 * the whole of it has been read. */
export function Writing({
  waiting,
  pullRequest,
  publish,
  onSent,
  onError,
  onCancel,
}: WritingProps) {
  // With nothing waiting to go, approving is the only verdict on offer, so it
  // is also where the picker starts. A clean approval is the commonest one
  // there is, and the other two would have nothing to point at.
  const alone = waiting === 0;
  const [verdict, setVerdict] = useState<Verdict>(alone ? "approve" : "comment");
  const [summary, setSummary] = useState("");
  const [sending, setSending] = useState(false);

  // GitHub's rule, and a fair one: approving needs no words, and the other two
  // are somebody being asked to do something, where "why" is the whole of it.
  const needsSummary = verdict !== "approve";
  const short = needsSummary && !summary.trim();

  async function send() {
    if (short || sending) return;
    setSending(true);
    try {
      onSent(await publish(verdict, summary));
    } catch (e) {
      onError(String(e));
    } finally {
      setSending(false);
    }
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle className="font-sans text-base text-ink">Send review</DialogTitle>
        <DialogDescription className="font-serif text-ink-soft">
          {alone
            ? "No comments are waiting to go."
            : `${waiting} comment${waiting === 1 ? "" : "s"} will go with it.`}{" "}
          {pullRequest !== undefined && `Pull request #${pullRequest}.`}
        </DialogDescription>
      </DialogHeader>

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

      <Verdicts value={verdict} onChange={setVerdict} approveOnly={alone} />

      <DialogFooter>
        <Button size="sm" variant="ghost" className="cursor-pointer text-muted" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          size="sm"
          className="cursor-pointer"
          disabled={short || sending}
          title={short ? "A review that asks for something has to say what" : undefined}
          onClick={() => void send()}
        >
          {sending ? "Sending…" : "Send"}
        </Button>
      </DialogFooter>
    </>
  );
}

type WritingProps = {
  /** Comments that have not gone yet, which is what is about to be sent. */
  waiting: number;
  pullRequest?: number;
  publish: (verdict: Verdict, summary: string) => Promise<SentView>;
  onSent: (sent: SentView) => void;
  onError: (message: string) => void;
  onCancel: () => void;
};
