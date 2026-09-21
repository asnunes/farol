import { useRef } from "react";
import { useCurrentFile } from "@/hooks/useCurrentFile";
import { useDiffs } from "@/hooks/useDiffs";
import { useResumeAt } from "@/hooks/useResumeAt";
import { BlockBar } from "@/review/BlockBar";
import { FileSection } from "@/review/FileSection";
import { Unmapped } from "@/review/Unmapped";
import type { CommentActions } from "@/hooks/useComments";
import type { DiffView } from "@/hooks/useDiffView";
import type { OpenFiles } from "@/hooks/useOpenFiles";
import { readingRows } from "@/api";
import type { ReviewView } from "@/api";

/** The whole review, in one scroll.
 *
 * Every block and every file is on the page in reading order, the way the
 * session laid them out, rather than one file at a time. The blocks and the
 * files they hold are laid end to end as one list, so a file is drawn by the
 * same lines wherever it came from. */
export function Reading({
  review,
  view,
  current,
  onCurrent,
  onToggleViewed,
  files,
  comments,
}: ReadingProps) {
  const pane = useRef<HTMLElement>(null);
  const { diffs, errors, request, retry } = useDiffs();

  // Landing first, and the order is load-bearing: effects run in the order
  // they are called, and naming the current file from the scroll before the
  // page has been scrolled names whatever sits at the top of a pane nobody has
  // moved yet. That name then outlives the landing, because the correction that
  // follows the scroll is suppressed by the settling the scroll itself set.
  useResumeAt(current);
  useCurrentFile(pane, review, onCurrent);

  // Nothing left in the comparison: the base moved under the map, typically
  // because the branch was merged. The blocks are still on disk and still in
  // the sidebar; there is simply no diff to read, and saying so beats a page of
  // empty block bars over files that would never load.
  if (review.totalFiles === 0 && review.unmapped.length === 0) {
    return (
      <main ref={pane} className="pane grid place-items-center bg-ground" data-current="">
        <p className="empty font-serif text-sm text-ink-muted">No changes in this comparison.</p>
      </main>
    );
  }

  return (
    <main ref={pane} className="pane overflow-y-auto bg-ground" data-current={current ?? ""}>
      {readingRows(review).map((row) =>
        "block" in row ? (
          <BlockBar
            key={row.block.slug}
            block={row.block}
            number={row.number}
            total={review.blocks.length}
          />
        ) : (
          <FileSection
            key={row.file.path}
            file={row.file}
            diff={diffs[row.file.path]}
            error={errors[row.file.path]}
            onRetry={() => retry(row.file.path)}
            view={view}
            open={files.isOpen(row.file)}
            onToggleOpen={() => files.set(row.file.path, !files.isOpen(row.file))}
            onReach={() => request(row.file.path)}
            onToggleViewed={() => onToggleViewed(row.file.path, !row.file.viewed)}
            comments={comments.on(row.file.path)}
            commentActions={comments}
          />
        ),
      )}

      {review.unmapped.length > 0 && <Unmapped paths={review.unmapped} />}

      {/* Room under the last file, so it too can be brought up to the line a
          quarter down the pane that decides which file is being read.
          
          A block and not the padding this was: padding belongs to the pane's
          own box, so a pane shorter than the padding — which is what a narrow
          screen with the sidebar open leaves — grew past the row it was given
          and printed over the key bar. Scrollable content cannot do that.
          
          Three quarters of the pane and not a share of the window: taller than
          the pane, it scrolls the last file off the top instead of up to the
          line, and the review ends up reporting a file nobody is looking at. */}
      <div aria-hidden="true" className="tail h-3/4" />
    </main>
  );
}

type ReadingProps = {
  review: ReviewView;
  view: DiffView;
  current: string | null;
  onCurrent: (path: string) => void;
  onToggleViewed: (path: string, viewed: boolean) => void;
  files: OpenFiles;
  comments: CommentActions;
};
