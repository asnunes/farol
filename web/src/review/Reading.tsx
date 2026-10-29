import { Fragment, useRef } from "react";
import { useCurrentFile } from "@/hooks/useCurrentFile";
import { useDiffs } from "@/hooks/useDiffs";
import { useResumeAt } from "@/hooks/useResumeAt";
import { BlockBar } from "@/review/BlockBar";
import { FileSection } from "@/review/FileSection";
import { DriftedThreads } from "@/review/DriftedThreads";
import { Unmapped } from "@/review/Unmapped";
import { BlockLenses } from "@/review/lenses/BlockLenses";
import { CodeBar } from "@/review/lenses/CodeBar";
import { ReviewLenses } from "@/review/lenses/ReviewLenses";
import { scrollToFile, scrollToLine } from "@/lib/scroll";
import type { CommentActions } from "@/hooks/useComments";
import type { DiffView } from "@/hooks/useDiffView";
import type { OpenFiles } from "@/hooks/useOpenFiles";
import { blockOf, linesOf, readingOrder, readingRows } from "@/api";
import type { CitationView, CommentView, LensView, ReviewView } from "@/api";

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
  onSeen,
}: ReadingProps) {
  const pane = useRef<HTMLElement>(null);
  const drifted = byBlock(review, comments.drifted);
  const { diffs, errors, request, retry } = useDiffs();

  // A citation in a lens: open its file, bring it up, and land on the line.
  const cite = (citation: CitationView) => {
    const file = readingOrder(review).find((f) => f.path === citation.path);
    if (!file) return;
    files.set(file, true);
    scrollToFile(file.path);
    scrollToLine(file.path, citation.side, citation.from);
  };

  // Landing first, and the order is load-bearing: effects run in the order
  // they are called, and naming the current file from the scroll before the
  // page has been scrolled names whatever sits at the top of a pane nobody has
  // moved yet. That name then outlives the landing, because the correction that
  // follows the scroll is suppressed by the settling the scroll itself set.
  useResumeAt(current, current === readingOrder(review)[0]?.path);
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
      <ReviewLenses lenses={review.lenses} onCite={cite} onSeen={onSeen} comments={comments} />
      {readingRows(review).map((row) =>
        "block" in row ? (
          <Fragment key={row.block.slug}>
            <BlockBar block={row.block} number={row.number} total={review.blocks.length} comments={comments} />
            <BlockLenses slug={row.block.slug} lenses={row.block.lenses} onCite={cite} onSeen={onSeen} comments={comments} />
            {row.block.files.length > 0 && <CodeBar block={row.block} />}
          </Fragment>
        ) : (
          <Fragment key={row.file.path}>
            <FileSection
              file={row.file}
              diff={diffs[row.file.path]}
              error={errors[row.file.path]}
              onRetry={() => retry(row.file.path)}
              view={view}
              open={files.isOpen(row.file)}
              onToggleOpen={() => files.set(row.file, !files.isOpen(row.file))}
              onReach={() => request(row.file.path)}
              onToggleViewed={() => onToggleViewed(row.file.path, !row.file.viewed)}
              comments={comments.on(row.file.path)}
              commentActions={comments}
              onCite={cite}
            />
            {/* After the block's last file: the threads of its files that the
                diff no longer reaches. */}
            <DriftedThreads threads={drifted.lastOf.get(row.file.path) ?? []} actions={comments} />
          </Fragment>
        ),
      )}

      {review.unmapped.length > 0 && <Unmapped paths={review.unmapped} />}

      <DriftedThreads
        threads={[...drifted.homeless, ...unplaced(review, comments.all)]}
        actions={comments}
        heading="Threads on something no longer in the review"
      />

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

/** Threads on a lens or a block the page does not draw: a block renamed or
 * dropped, a lens switched off or written empty. Without this they would count
 * in the activity box and be drawn nowhere, with no way to answer or close
 * them. */
function unplaced(review: ReviewView, threads: CommentView[]): CommentView[] {
  const lenses = new Set([
    ...review.lenses.map((l) => `pr:${l.id}`),
    ...review.blocks.flatMap((b) => [
      ...b.lenses.map((l) => `block:${b.slug}:${l.id}`),
      ...b.files.flatMap((f) => f.lenses.map((l) => `file:${f.path}:${l.id}`)),
    ]),
  ]);
  const blocks = new Set(review.blocks.map((b) => b.slug));
  return threads.filter(
    (t) => (t.about.kind === "lens" && !lenses.has(t.about.key)) || (t.about.kind === "block" && !blocks.has(t.about.slug)),
  );
}

/** Where each thread off the diff goes: after the last file of the block its
 * file is read in, or, with no such block, into the block of their own. */
function byBlock(review: ReviewView, threads: CommentView[]): Placement {
  const lastOf = new Map<string, CommentView[]>();
  const homeless: CommentView[] = [];
  for (const thread of threads) {
    const home = blockOf(review, linesOf(thread)?.path ?? "")?.block;
    const last = home?.files[home.files.length - 1]?.path;
    if (!last) {
      homeless.push(thread);
      continue;
    }
    lastOf.set(last, [...(lastOf.get(last) ?? []), thread]);
  }
  return { lastOf, homeless };
}

type Placement = {
  /** By the path of the block's last file, which they are drawn after. */
  lastOf: Map<string, CommentView[]>;
  homeless: CommentView[];
};

type ReadingProps = {
  review: ReviewView;
  view: DiffView;
  current: string | null;
  onCurrent: (path: string) => void;
  onToggleViewed: (path: string, viewed: boolean) => void;
  files: OpenFiles;
  comments: CommentActions;
  onSeen: (lens: LensView, seen: boolean) => void;
};
