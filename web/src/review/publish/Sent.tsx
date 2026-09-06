import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { SentView } from "@/api";

/** What happened, once it has happened.
 *
 * Two things can, and they are not the same thing: a review was sent, or the
 * ticks went up on their own. Saying "review sent" for the second is telling
 * somebody they did a thing they deliberately did not do.
 *
 * Short on purpose. The reader pressed the button and knows what they asked
 * for; what they cannot know is the address it landed at and whether the parts
 * that could fail did. */
export function Sent({ done, pullRequest, onDone }: SentProps) {
  return (
    <>
      <DialogHeader>
        <DialogTitle className="font-sans text-base text-ink">
          {"review" in done ? "Review sent" : "Files ticked"}
        </DialogTitle>
        <DialogDescription className="font-serif text-ink-soft">
          {"review" in done ? said(done.review) : ticked(done.ticks, pullRequest)}
        </DialogDescription>
      </DialogHeader>

      {"review" in done && (
        <a
          className="font-mono text-sm break-all text-highlight underline"
          href={done.review.url}
          target="_blank"
          rel="noreferrer"
        >
          {done.review.url}
        </a>
      )}

      <DialogFooter>
        <Button size="sm" onClick={onDone}>
          Done
        </Button>
      </DialogFooter>
    </>
  );
}

/** The two things a review can leave behind that the reader cannot see from
 * here: what went with it, and what did not. */
function said(sent: SentView): string {
  const parts = [];
  if (sent.comments > 0) {
    parts.push(
      `${sent.comments} comment${sent.comments === 1 ? "" : "s"} went with it, and stay${
        sent.comments === 1 ? "s" : ""
      } here until you close ${sent.comments === 1 ? "it" : "them"}.`,
    );
  }
  if (sent.readFailed !== null) {
    parts.push(`The files you had read were not ticked: ${sent.readFailed}`);
  } else if (sent.read > 0) {
    parts.push(`${sent.read} file${sent.read === 1 ? "" : "s"} ticked.`);
  }
  return parts.join(" ");
}

function ticked(read: number, pullRequest?: number): string {
  const where = pullRequest === undefined ? "the pull request" : `pull request #${pullRequest}`;
  return `${read} file${read === 1 ? "" : "s"} ticked on ${where}. No review was sent.`;
}

/** Which of the two happened. */
export type Done = { review: SentView } | { ticks: number };

type SentProps = {
  done: Done;
  pullRequest?: number;
  onDone: () => void;
};
