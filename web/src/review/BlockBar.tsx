import type { BlockView } from "@/api";

/** The band above the diff: which block you are in and why it exists. */
export function BlockBar({
  block,
  number,
  total,
}: {
  block: BlockView;
  number: number;
  total: number;
}) {
  return (
    <div className="blockbar border-b border-rule bg-surface px-6 py-4">
      <div className="kicker font-mono text-[0.6875rem] tracking-wide text-faint uppercase">
        block {number} of {total}
      </div>
      <h2 className="mt-1 font-sans text-lg font-semibold text-ink">{block.title}</h2>
      {block.context && (
        <p className="mt-2 max-w-[68ch] font-serif text-[0.9375rem] leading-relaxed text-ink-soft">
          {block.context}
        </p>
      )}
    </div>
  );
}
