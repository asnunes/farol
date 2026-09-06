import { useState } from "react";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Sent } from "./Sent";
import { Writing } from "./Writing";
import type { Done } from "./Sent";
import type { SentView, Verdict } from "@/api";

/** The last step of a review, as one dialog with two faces: writing it, and
 * where it went.
 *
 * The two share the frame and nothing else. Each brings its own title, its own
 * words and its own buttons, which is why neither is a branch inside the other:
 * this decides which face is showing, and stops there. */
export function Send({
  open,
  onOpenChange,
  waiting,
  read,
  mine,
  pullRequest,
  publish,
  onTicks,
  onError,
}: SendProps) {
  const [done, setDone] = useState<Done | null>(null);

  function close(next: boolean) {
    onOpenChange(next);
    // Forgotten on the way out. Reopening is a new review, not the last one
    // still on the screen, and what was typed goes with the panel that held it.
    if (!next) setDone(null);
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="send-card border-rule bg-surface sm:max-w-lg">
        {done ? (
          <Sent done={done} pullRequest={pullRequest} onDone={() => close(false)} />
        ) : (
          <Writing
            waiting={waiting}
            read={read}
            mine={mine}
            onTicks={onTicks}
            pullRequest={pullRequest}
            publish={publish}
            onSent={(sent) => setDone({ review: sent })}
            onTicked={(read) => setDone({ ticks: read })}
            onError={(message) => {
              onError(message);
              close(false);
            }}
            onCancel={() => close(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

type SendProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Comments that have not gone yet, which is what is about to be sent. */
  waiting: number;
  /** Files the reviewer has read, which go up ticked with the review. */
  read: number;
  /** The reviewer opened this pull request, so only a comment can go on it. */
  mine: boolean;
  pullRequest?: number;
  publish: (verdict: Verdict, summary: string) => Promise<SentView>;
  /** Send the ticks with no review in front of them. */
  onTicks: () => Promise<number>;
  onError: (message: string) => void;
};
