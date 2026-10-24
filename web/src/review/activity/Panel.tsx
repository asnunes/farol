import { X } from "lucide-react";
import { Section } from "./Section";
import { UpdateRow } from "./UpdateRow";
import { WorkingRow } from "./WorkingRow";
import type { Activity } from "@/hooks/useActivity";
import type { CommentView } from "@/api";

/** The box open: what the agents owe, then what they did. */
export function Panel({ activity, onJump, onClose }: PanelProps) {
  const { working, updates, fresh } = activity;

  return (
    <div
      role="dialog"
      aria-label="Agent activity"
      className="panel flex max-h-[26rem] w-[23.75rem] max-w-[calc(100vw-2rem)] flex-col overflow-hidden rounded-[10px] border border-rule-strong bg-surface shadow-[0_12px_32px_rgb(0_0_0/0.45)]"
    >
      <div className="flex items-center gap-2 border-b border-rule px-3 py-2 font-sans text-[0.8125rem]">
        <b className="font-semibold text-ink">Agent activity</b>
        <span className="flex-1" />
        {fresh > 0 && (
          <button className="font-sans text-xs text-comment-faint hover:text-comment-ink" onClick={activity.readAll}>
            Mark all read
          </button>
        )}
        <button
          className="px-1 text-faint hover:text-ink"
          aria-label="Close agent activity"
          title="Close — ."
          onClick={onClose}
        >
          <X className="size-3.5" aria-hidden="true" />
        </button>
      </div>

      <ul className="overflow-x-hidden overflow-y-auto p-1">
        {working.length > 0 && <Section>In progress</Section>}
        {working.map((w) => (
          <WorkingRow key={w.thread.id} item={w} onJump={() => onJump(w.thread)} />
        ))}
        {updates.length > 0 && <Section>Updates</Section>}
        {updates.map((u) => (
          <UpdateRow
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
  );
}

type PanelProps = {
  activity: Activity;
  onJump: (thread: CommentView) => void;
  onClose: () => void;
};
