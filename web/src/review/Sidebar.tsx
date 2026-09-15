import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { fileLabels } from "@/lib/path";
import { FileRow } from "./FileRow";
import { commentsOn } from "./diff/line";
import { readingOrder } from "@/api";
import type { FileLabels } from "@/lib/path";
import type { BlockView, CommentView, ReviewView } from "@/api";

/** Navigation only, deliberately: no prose here, or the reader would try to
 * read the map instead of the code. */
export function Sidebar({
  review,
  comments,
  current,
  onPick,
}: SidebarProps) {
  // Over the sidebar as a whole, not per block: two files called `mod.rs` are
  // two rows to tell apart wherever they were grouped.
  const label = useMemo(
    () => fileLabels(readingOrder(review).map((f) => f.path)),
    [review],
  );

  return (
    <aside className="sidebar overflow-y-auto border-r border-rule bg-surface py-3">
      {review.blocks.map((block, i) => (
        <Block
          key={block.slug}
          block={block}
          label={label}
          comments={comments}
          number={i + 1}
          current={current}
          onPick={onPick}
        />
      ))}

      {review.looseSkim.length > 0 && (
        <section className="blk mb-4 px-3" data-state="loose">
          <div className="blk-head flex items-center gap-2 px-2">
            <div
              aria-hidden="true"
              className="num loose grid size-5 shrink-0 place-items-center rounded-full bg-sunken font-mono text-[0.6875rem] text-faint"
            >
              ~
            </div>
            <div className="blk-title loose truncate font-sans text-[0.8125rem] text-ink-muted italic">
              No block · safe to skim
            </div>
          </div>
          <ul className="blk-files mt-1">
            {review.looseSkim.map((f) => (
              <FileRow
                key={f.path}
                file={f}
                label={label(f.path)}
                comments={commentsOn(comments, f.path).length}
                current={current}
                onPick={onPick}
              />
            ))}
          </ul>
        </section>
      )}
    </aside>
  );
}

function Block({
  block,
  label,
  comments,
  number,
  current,
  onPick,
}: BlockProps) {
  const state =
    block.files.length > 0 && block.files.every((f) => f.viewed)
      ? "done"
      : block.files.some((f) => f.path === current)
        ? "current"
        : "todo";

  return (
    <section className="blk mb-4 px-3" data-state={state}>
      <div className="blk-head flex items-center gap-2 px-2">
        <div
          aria-hidden="true"
          className={cn(
            "num grid size-5 shrink-0 place-items-center rounded-full font-mono text-[0.6875rem]",
            state === "done" && "bg-add-bg text-add-ink",
            state === "current" && "bg-highlight text-surface",
            state === "todo" && "bg-sunken text-ink-muted",
          )}
        >
          {state === "done" ? "✓" : number}
        </div>
        <div
          className={cn(
            "blk-title truncate font-sans text-[0.8125rem] font-semibold text-ink",
            state === "done" && "text-ink-muted",
          )}
        >
          {block.title}
        </div>
      </div>
      <ul className="blk-files mt-1">
        {block.files.map((f) => (
          <FileRow
            key={f.path}
            file={f}
            label={label(f.path)}
            comments={commentsOn(comments, f.path).length}
            current={current}
            onPick={onPick}
          />
        ))}
      </ul>
    </section>
  );
}

type SidebarProps = {
  review: ReviewView;
  comments: CommentView[];
  current: string | null;
  onPick: (path: string) => void;
};

type BlockProps = {
  block: BlockView;
  label: FileLabels;
  comments: CommentView[];
  number: number;
  current: string | null;
  onPick: (path: string) => void;
};
