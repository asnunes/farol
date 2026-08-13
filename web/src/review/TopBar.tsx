import { TriangleAlert } from "lucide-react";
import { Progress } from "@/components/ui/progress";
import { Refresh } from "@/review/Refresh";
import { ViewToggle } from "@/review/ViewToggle";
import type { DiffView } from "@/hooks/useDiffView";
import type { ReviewView } from "@/api";

/** Where you are and how far through you are. */
export function TopBar({ review, view, onView, stale, onRefresh, unreadable }: TopBarProps) {
  const done = review.totalFiles > 0 && review.viewedFiles === review.totalFiles;

  return (
    <header className="top col-span-full flex flex-wrap items-center justify-between gap-4 border-b border-rule bg-surface px-5 py-2.5">
      <div className="refs flex items-baseline gap-2 font-mono text-[0.8125rem]">
        <span className="head font-semibold">{review.branch}</span>
        <span className="text-rule-strong">→</span>
        <span className="base text-muted">{review.base}</span>
      </div>

      <div className="flex items-center gap-4">
        {unreadable.length > 0 && <Unreadable files={unreadable} />}
        {stale && <Refresh onRefresh={onRefresh} />}
        <ViewToggle view={view} onChange={onView} />
        {review.commitsBehind > 0 && (
          <div className="stale-chip rounded-full bg-accent-dim px-2.5 py-1 font-mono text-xs text-accent">
            map {review.commitsBehind} commit{review.commitsBehind === 1 ? "" : "s"} behind
          </div>
        )}
        <div className="progress flex items-center gap-2 font-mono text-xs text-muted">
          {done ? (
            // The only celebration in the app, and only once there is nothing
            // left to read.
            <span className="done" title="Everything read">
              🎉
            </span>
          ) : (
            <span>
              {review.viewedFiles} / {review.totalFiles}
            </span>
          )}
          <Progress
            className="meter h-1.5 w-28 bg-sunken"
            value={review.totalFiles ? (review.viewedFiles / review.totalFiles) * 100 : 0}
            aria-label={`${review.viewedFiles} of ${review.totalFiles} read`}
          />
        </div>
      </div>
    </header>
  );
}

/** Comment files the store could not read.
 *
 * Up here rather than beside the code, because a comment whose header is broken
 * has no line left to sit next to — that is exactly what is wrong with it. The
 * chip names the files in its tooltip, since fixing one means opening it. */
function Unreadable({ files }: { files: string[] }) {
  return (
    <div
      className="unreadable flex items-center gap-1.5 rounded-full bg-del-bg px-2.5 py-1 font-mono text-xs text-del-ink"
      title={`Could not be read — the header needs path: and lines: between two --- lines.\n\n${files.join("\n")}`}
    >
      <TriangleAlert className="size-3.5" aria-hidden="true" />
      {files.length} comment file{files.length === 1 ? "" : "s"} unreadable
    </div>
  );
}

type TopBarProps = {
  review: ReviewView;
  view: DiffView;
  onView: (view: DiffView) => void;
  stale: boolean;
  onRefresh: () => void;
  /** Paths of comment files that could not be parsed. */
  unreadable: string[];
};
