import { useRef } from "react";
import { Launcher } from "./Launcher";
import { Panel } from "./Panel";
import type { Activity as ActivityState } from "@/hooks/useActivity";
import type { CommentView } from "@/api";

/** What the agents did on the threads, in the corner, so the reader can keep
 * reading while one works.
 *
 * Closed, it says what is new and what is under way. Open, it lists both, and a
 * row takes the reader to its thread — opening the file and the thread if they
 * were folded away. It stays open after a jump, for the reader going through
 * several answers in a row. */
export function Activity({ activity, open, onOpen, onJump }: ActivityProps) {
  const launcher = useRef<HTMLButtonElement>(null);

  return (
    <div className="activity fixed right-4 bottom-12 z-40 flex flex-col items-end gap-2">
      {open && (
        <Panel
          activity={activity}
          onJump={onJump}
          onClose={() => {
            onOpen(false);
            launcher.current?.focus();
          }}
        />
      )}
      <Launcher ref={launcher} activity={activity} open={open} onToggle={() => onOpen(!open)} />
    </div>
  );
}

type ActivityProps = {
  activity: ActivityState;
  open: boolean;
  onOpen: (open: boolean) => void;
  /** Take the reader to a thread, opening its file and the thread itself. */
  onJump: (thread: CommentView) => void;
};
