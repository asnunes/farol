import { Progress } from "@/components/ui/progress";
import { ViewToggle } from "@/review/ViewToggle";
import type { DiffView } from "@/hooks/useDiffView";
import type { ReviewView } from "@/api";

/** Where you are and how far through you are. */
export function TopBar({ review, view, onView }: TopBarProps) {
  const done = review.totalFiles > 0 && review.viewedFiles === review.totalFiles;

  return (
    <header className="top col-span-full flex flex-wrap items-center justify-between gap-4 border-b border-rule bg-surface px-5 py-2.5">
      <div className="refs flex items-baseline gap-2 font-mono text-[0.8125rem]">
        <span className="head font-semibold">{review.branch}</span>
        <span className="text-rule-strong">→</span>
        <span className="base text-muted">{review.base}</span>
      </div>

      <div className="flex items-center gap-4">
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

type TopBarProps = {
  review: ReviewView;
  view: DiffView;
  onView: (view: DiffView) => void;
};
