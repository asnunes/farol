import { Badge } from "@/components/ui/badge";
import { splitPath } from "@/lib/path";
import type { FileView } from "@/api";

/** The file being read: the tick on the left, the path, the churn on the right. */
export function FileHeader({
  file,
  onToggleViewed,
}: {
  file: FileView;
  onToggleViewed: () => void;
}) {
  const { dir, name } = splitPath(file.path);

  return (
    <div className="filehead flex items-center justify-between gap-4 border-b border-rule bg-surface px-6 py-2.5">
      <div className="left flex min-w-0 items-center gap-3">
        <button
          className="markbox grid size-5 shrink-0 place-items-center rounded border border-rule-strong text-xs text-transparent transition-colors hover:border-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none aria-pressed:border-accent aria-pressed:bg-accent aria-pressed:text-surface"
          aria-pressed={file.viewed}
          title="Mark as read — key e"
          onClick={onToggleViewed}
        >
          <span aria-hidden="true">✓</span>
        </button>

        <div className="path truncate font-mono text-sm font-semibold text-ink">
          <span className="dir font-normal text-faint">{dir}</span>
          {name}
        </div>

        {/* Only worth showing when the file belongs to more than one block —
            otherwise the tag says what the block band already said. */}
        {file.tags.length > 1 && (
          <div className="tags flex shrink-0 gap-1">
            {file.tags.map((t) => (
              <Badge
                key={t}
                variant="secondary"
                className="tag border-transparent bg-sunken font-mono text-[0.6875rem] font-normal text-muted"
              >
                {t}
              </Badge>
            ))}
          </div>
        )}
      </div>

      <div className="churn shrink-0 font-mono text-xs">
        <span className="a text-add-ink">+{file.additions}</span>{" "}
        <span className="d text-del-ink">−{file.deletions}</span>
      </div>
    </div>
  );
}
