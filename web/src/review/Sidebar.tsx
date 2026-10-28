import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { fileLabels } from "@/lib/path";
import { FileRow } from "./FileRow";
import { LensRow } from "./LensRow";
import { readingOrder } from "@/api";
import type { CommentsOn } from "@/hooks/useComments";
import type { FileLabels } from "@/lib/path";
import type { BlockView, ReviewView } from "@/api";

/** Navigation only, deliberately: no prose here, or the reader would try to
 * read the map instead of the code.
 *
 * Beside the code where there is room for it, and above the code where there is
 * not — a column 19rem wide on a 390px screen leaves the diff a word per line.
 * Above, it is held to a third of the height: the point of opening it is to
 * reach a file, and a list that fills the screen has stopped being navigation
 * and become the page. It scrolls inside that third, and picking a file puts it
 * away, which is the composition's business and not this one's. */
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
    <aside className="sidebar overflow-y-auto border-rule bg-surface py-3 max-md:max-h-[33dvh] max-md:border-b md:border-r">
      {review.lenses.length > 0 && (
        <section className="blk mb-4 px-3" data-state="lenses">
          <div className="blk-head flex items-center gap-2 px-2">
            <div className="blk-title truncate font-sans text-[0.8125rem] font-semibold text-ink">Pull request</div>
          </div>
          <ul className="blk-files mt-1">
            {review.lenses.map((lens) => (
              <LensRow key={lens.id} lens={lens} current={current === `lens:pr:${lens.id}`} onPick={() => onPick(`lens:pr:${lens.id}`)} />
            ))}
          </ul>
        </section>
      )}
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
  // Read, not worked out: the rendered list is not what the block holds, and
  // counting it was how a block that renders nothing stayed unfinishable while
  // one that renders half its files reported done. Which file the reader is on
  // stays here, because the server has no way to know it.
  const state =
    block.viewedFiles === block.totalFiles
      ? "done"
      : block.files.some((f) => f.path === current) || current?.startsWith(`lens:block:${block.slug}:`)
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
        {block.lenses.map((lens) => (
          <LensRow
            key={lens.id}
            lens={lens}
            current={current === `lens:block:${block.slug}:${lens.id}`}
            onPick={() => onPick(`lens:block:${block.slug}:${lens.id}`)}
          />
        ))}
        {block.files.map((f) => (
          <FileRow
            key={f.path}
            file={f}
            label={label(f.path)}
            comments={comments(f.path).length}
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
  /** A file's comments, from the one place they are grouped. */
  comments: CommentsOn;
  current: string | null;
  /** A file path, or `lens:<key>` for a lens row. */
  onPick: (place: string) => void;
};

type BlockProps = {
  block: BlockView;
  label: FileLabels;
  comments: CommentsOn;
  number: number;
  current: string | null;
  onPick: (path: string) => void;
};
