import { useEffect, useRef, useState } from "react";
import { Send, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { ago, where, type Activity as ActivityState, type Update, type Working } from "@/hooks/useActivity";
import type { CommentView } from "@/api";

/** What the agents did on the threads, in the corner, so the reader can keep
 * reading while one works.
 *
 * Closed, it says what is new and what is under way. Open, it lists both, and a
 * row takes the reader to its thread — opening the file and the thread if they
 * were folded away. It stays open after a jump, for the reader going through
 * several answers in a row. */
export function Activity({ activity, open, onOpen, onJump }: ActivityProps) {
  const { working, updates, fresh } = activity;
  const started = working.filter((w) => w.started).length;
  const sent = working.length - started;
  const launcher = useRef<HTMLButtonElement>(null);
  const [bump, setBump] = useState(false);
  const before = useRef(fresh);

  // Something new arriving while the box is closed gets a small hop, which is
  // the whole of the interruption: the reader decides when to look.
  useEffect(() => {
    if (fresh > before.current) {
      setBump(true);
      const settle = window.setTimeout(() => setBump(false), 600);
      before.current = fresh;
      return () => window.clearTimeout(settle);
    }
    before.current = fresh;
  }, [fresh]);

  return (
    <div className="activity fixed right-4 bottom-12 z-40 flex flex-col items-end gap-2">
      {open && (
        <div
          role="dialog"
          aria-label="Agent activity"
          className="panel flex max-h-[26rem] w-[23.75rem] max-w-[calc(100vw-2rem)] flex-col overflow-hidden rounded-[10px] border border-rule-strong bg-surface shadow-[0_12px_32px_rgb(0_0_0/0.45)]"
        >
          <div className="flex items-center gap-2 border-b border-rule px-3 py-2 font-sans text-[0.8125rem]">
            <b className="font-semibold text-ink">Agent activity</b>
            <span className="flex-1" />
            {fresh > 0 && (
              <button
                className="font-sans text-xs text-comment-faint hover:text-comment-ink"
                onClick={activity.readAll}
              >
                Mark all read
              </button>
            )}
            <button
              className="px-1 text-faint hover:text-ink"
              aria-label="Close agent activity"
              title="Close — ."
              onClick={() => {
                onOpen(false);
                launcher.current?.focus();
              }}
            >
              <X className="size-3.5" aria-hidden="true" />
            </button>
          </div>

          <ul className="overflow-x-hidden overflow-y-auto p-1">
            {working.length > 0 && <Section>In progress</Section>}
            {working.map((w) => (
              <Row key={w.thread.id} item={w} onJump={() => onJump(w.thread)} />
            ))}
            {updates.length > 0 && <Section>Updates</Section>}
            {updates.map((u) => (
              <Row
                key={u.key}
                item={u}
                onJump={() => {
                  activity.read(u);
                  onJump(u.thread);
                }}
              />
            ))}
            {working.length === 0 && updates.length === 0 && (
              <li className="px-4 py-5 text-center font-sans text-[0.8125rem] text-ink-muted">
                Nothing yet. Answers from the agent show up here.
              </li>
            )}
          </ul>
        </div>
      )}

      <button
        ref={launcher}
        className={cn(
          "launcher flex items-center gap-2 rounded-full border border-rule-strong bg-surface px-3 py-1.5 font-sans text-[0.78rem] text-ink-soft shadow-[0_6px_20px_rgb(0_0_0/0.4)] hover:text-ink",
          bump && "animate-[activity-bump_0.6s_ease]",
        )}
        aria-expanded={open}
        title="Agent activity — ."
        onClick={() => onOpen(!open)}
      >
        <span className={cn("size-[7px] rounded-full", fresh > 0 ? "bg-highlight" : "bg-faint")} aria-hidden="true" />
        {fresh === 0 && working.length === 0 && <span>No agent activity</span>}
        {fresh > 0 && <b className="font-semibold text-ink">{fresh} new</b>}
        {fresh > 0 && working.length > 0 && <span aria-hidden="true">·</span>}
        {sent > 0 && (
          <span className="inline-flex items-center gap-1.5 text-comment-ink">
            <Send className="size-3" aria-hidden="true" />
            {sent} waiting
          </span>
        )}
        {sent > 0 && started > 0 && <span aria-hidden="true">·</span>}
        {started > 0 && (
          <span className="inline-flex items-center gap-1.5 text-comment-ink">
            <Spinner />
            {started} working
          </span>
        )}
      </button>
    </div>
  );
}

/** One row: who did what, where, how long ago, and the start of it. */
function Row({ item, onJump }: RowProps) {
  const isWorking = !("key" in item);
  const read = !isWorking && item.read;
  const when = isWorking ? `since ${ago(item.since).replace(" ago", "")}` : ago(item.at);
  const line = isWorking ? item.line : `${item.who} ${item.what}`;

  return (
    <li>
      <button
        className="grid w-full grid-cols-[14px_minmax(0,1fr)] gap-2 rounded-[7px] px-2 py-2 text-left hover:bg-sunken"
        onClick={onJump}
      >
        {isWorking ? (
          <span className="mt-[5px] text-comment-ink">
            {item.started ? <Spinner /> : <Send className="size-3" aria-hidden="true" />}
          </span>
        ) : (
          <span
            className={cn(
              "mt-1.5 size-[7px] rounded-full",
              read ? "border border-rule-strong" : "bg-highlight",
            )}
            aria-label={read ? undefined : "new"}
          />
        )}
        <span className="min-w-0">
          <span className={cn("block font-sans text-[0.8125rem]", read ? "text-ink-muted" : "text-ink")}>
            {line}
          </span>
          <span className="block font-mono text-[0.72rem] text-comment-ink">
            {where(item.thread)} <span className="text-faint">· {when}</span>
          </span>
          <span className="block truncate font-sans text-xs text-ink-muted">{item.excerpt}</span>
        </span>
      </button>
    </li>
  );
}

function Section({ children }: SectionProps) {
  return (
    <li className="px-2.5 pt-2 pb-0.5 font-sans text-[0.6875rem] tracking-wider text-faint uppercase">
      {children}
    </li>
  );
}

function Spinner() {
  return (
    <span
      className="inline-block size-[11px] animate-spin rounded-full border-2 border-comment-ink border-r-transparent"
      aria-hidden="true"
    />
  );
}

type ActivityProps = {
  activity: ActivityState;
  open: boolean;
  onOpen: (open: boolean) => void;
  /** Take the reader to a thread, opening its file and the thread itself. */
  onJump: (thread: CommentView) => void;
};

type RowProps = { item: Update | Working; onJump: () => void };

type SectionProps = { children: React.ReactNode };
