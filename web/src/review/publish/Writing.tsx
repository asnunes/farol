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
import { said } from "@/lib/utils";

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

  // Only the verdict that asks for work needs words of its own. Comment and
  // approve are carried by whatever is going with them, and farol sends the
  // comments as a draft first so that the host asks for no summary either.
  const short = verdict === "requestChanges" && !summary.trim();

  async function send() {
    if (short || sending) return;
    setSending(true);
    try {
      onSent(await publish(verdict, summary));
    } catch (e) {
      onError(said(e));
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
        placeholder={ASKS[verdict]}
        value={summary}
        disabled={sending}
        onChange={(e) => setSummary(e.target.value)}
      />

      <Verdicts value={verdict} onChange={setVerdict} approveOnly={alone} />

      <DialogFooter>
        <Button size="sm" variant="ghost" className="text-ink-muted" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          size="sm"
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

/** What the box is for, which is a different thing under each verdict. */
const ASKS: Record<Verdict, string> = {
  requestChanges: "What has to change before this can be merged. Markdown.",
  comment: "Anything to say on top of the comments. Optional.",
  approve: "Anything to add. Optional when approving.",
};

type WritingProps = {
  /** Comments that have not gone yet, which is what is about to be sent. */
  waiting: number;
  pullRequest?: number;
  publish: (verdict: Verdict, summary: string) => Promise<SentView>;
  onSent: (sent: SentView) => void;
  onError: (message: string) => void;
  onCancel: () => void;
};
