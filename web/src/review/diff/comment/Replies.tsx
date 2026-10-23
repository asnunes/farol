import { Prose } from "@/review/Prose";
import { UseAsPending } from "./UseAsPending";
import type { ReplyView } from "@/api";

/** The turns after the question, each signed by who wrote it. */
export function Replies({ replies, onUse }: RepliesProps) {
  return (
    <ol className="replies flex flex-col gap-2">
      {replies.map((reply, i) => (
        <li
          key={i}
          className={
            reply.from === "agent"
              ? "reply agent border-l-2 border-comment-ink pl-3"
              : "reply reviewer border-l-2 border-rule-strong pl-3"
          }
        >
          <span className="who block font-mono text-xs text-comment-ink">{reply.name}</span>
          <Prose className="body font-sans text-sm leading-relaxed text-ink-soft">{reply.body}</Prose>
          {onUse && <UseAsPending onUse={() => onUse(reply.body)} />}
        </li>
      ))}
    </ol>
  );
}

type RepliesProps = {
  replies: ReplyView[];
  /** Offered in a peer review: this turn, as the thread's pending comment. */
  onUse?: (body: string) => Promise<boolean>;
};
