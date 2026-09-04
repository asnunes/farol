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
  read,
  mine,
  pullRequest,
  publish,
  onTicks,
  onSent,
  onError,
  onCancel,
}: WritingProps) {
  // Commenting is where it starts, and on your own pull request it is where it
  // stays: GitHub takes a comment there and refuses the other two.
  const [verdict, setVerdict] = useState<Verdict>("comment");
  const [summary, setSummary] = useState("");
  const [sending, setSending] = useState(false);

  // Only the verdict that asks for work needs words of its own. Comment and
  // approve are carried by whatever is going with them, and farol sends the
  // comments as a draft first so that the host asks for no summary either.
  const short = verdict === "requestChanges" && !summary.trim();

  async function ticks() {
    if (sending) return;
    setSending(true);
    try {
      onSent({ url: "", comments: 0, read: await onTicks(), readFailed: null });
    } catch (e) {
      onError(String(e));
    } finally {
      setSending(false);
    }
  }

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
          {waiting === 0
            ? "No comments are waiting to go."
            : `${waiting} comment${waiting === 1 ? "" : "s"} will go with it.`}{" "}
          {read > 0 &&
            `The ${read} file${read === 1 ? "" : "s"} you have read ${
              read === 1 ? "goes" : "go"
            } up ticked. `}
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

      <Verdicts value={verdict} onChange={setVerdict} mine={mine} />

      <DialogFooter>
        {/* The ticks with no review in front of them. Offered whenever there
            are any, because the review that cannot be sent is exactly when
            somebody still wants the next round to show what changed. */}
        {read > 0 && (
          <Button
            size="sm"
            variant="ghost"
            className="mr-auto text-ink-muted"
            disabled={sending}
            onClick={() => void ticks()}
          >
            Just tick the files
          </Button>
        )}
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
  /** Files the reviewer has read, which go up ticked with it. */
  read: number;
  /** The reviewer opened this pull request, so only a comment can go on it. */
  mine: boolean;
  /** Send the ticks and no review. */
  onTicks: () => Promise<number>;
  pullRequest?: number;
  publish: (verdict: Verdict, summary: string) => Promise<SentView>;
  onSent: (sent: SentView) => void;
  onError: (message: string) => void;
  onCancel: () => void;
};
