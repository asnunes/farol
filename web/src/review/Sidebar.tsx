import { cn } from "@/lib/utils";
import { FileRow } from "./FileRow";
import type { BlockView, ReviewView } from "@/api";

/** Navigation only, deliberately: no prose here, or the reader would try to
 * read the map instead of the code. */
export function Sidebar({
  review,
  current,
  onPick,
}: SidebarProps) {
  return (
    <aside className="map overflow-y-auto border-r border-rule bg-surface py-3">
      {review.blocks.map((block, i) => (
        <Block
          key={block.slug}
          block={block}
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
              <FileRow key={f.path} file={f} current={current} onPick={onPick} />
            ))}
          </ul>
        </section>
      )}
    </aside>
  );
}

function Block({
  block,
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
            state === "current" && "bg-accent text-surface",
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
          <FileRow key={f.path} file={f} current={current} onPick={onPick} />
        ))}
      </ul>
    </section>
  );
}

type SidebarProps = {
  review: ReviewView;
  current: string | null;
  onPick: (path: string) => void;
};

type BlockProps = {
  block: BlockView;
  number: number;
  current: string | null;
  onPick: (path: string) => void;
};
