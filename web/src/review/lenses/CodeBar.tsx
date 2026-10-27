import type { BlockView } from "@/api";

/** Where a block's code starts, after what its lenses said: a thin line, and
 * then the files exactly as the page draws them anywhere else. */
export function CodeBar({ block }: CodeBarProps) {
  const added = block.files.reduce((sum, f) => sum + f.additions, 0);
  const removed = block.files.reduce((sum, f) => sum + f.deletions, 0);
  const count = block.files.length;
  return (
    <div className="codebar flex items-center gap-2.5 border-b border-rule bg-sunken px-4 py-1.5 text-xs text-ink-muted md:px-6">
      <span className="font-mono text-[0.6875rem] tracking-wide uppercase">code</span>
      <span>
        {count} {count === 1 ? "file" : "files"} · <span className="text-add-ink">+{added}</span>{" "}
        <span className="text-del-ink">−{removed}</span>
      </span>
    </div>
  );
}

type CodeBarProps = { block: BlockView };
