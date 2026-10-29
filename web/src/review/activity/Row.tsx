import { cn } from "@/lib/utils";
import { where } from "@/review/diff/comment/quoted";
import type { CommentView } from "@/api";

/** One line of the box, whatever it reports: a mark, what happened, where and
 * when, and the start of what was said. Pressing it is the jump. */
export function Row({ mark, line, thread, when, excerpt, quiet, onJump }: RowProps) {
  return (
    <li>
      <button
        className="grid w-full grid-cols-[14px_minmax(0,1fr)] gap-2 rounded-[7px] px-2 py-2 text-left hover:bg-sunken"
        onClick={onJump}
      >
        {mark}
        <span className="min-w-0">
          <span className={cn("block font-sans text-[0.8125rem]", quiet ? "text-ink-muted" : "text-ink")}>
            {line}
          </span>
          <span className="block font-mono text-[0.72rem] text-comment-ink">
            {where(thread)} <span className="text-faint">· {when}</span>
          </span>
          <span className="block truncate font-sans text-xs text-ink-muted">{excerpt}</span>
        </span>
      </button>
    </li>
  );
}

type RowProps = {
  mark: React.ReactNode;
  line: string;
  thread: CommentView;
  when: string;
  excerpt: string;
  /** Already read: drawn quieter, so what is new stands out. */
  quiet: boolean;
  onJump: () => void;
};
