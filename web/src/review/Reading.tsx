import { useRef } from "react";
import { useCurrentFile } from "@/hooks/useCurrentFile";
import { useDiffs } from "@/hooks/useDiffs";
import { useResumeAt } from "@/hooks/useResumeAt";
import { BlockBar } from "@/review/BlockBar";
import { FileSection } from "@/review/FileSection";
import { Unmapped } from "@/review/Unmapped";
import type { DiffView } from "@/hooks/useDiffView";
import type { BlockView, FileView, ReviewView } from "@/api";

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
  onError,
}: ReadingProps) {
  const pane = useRef<HTMLElement>(null);
  const { diffs, request } = useDiffs(onError);

  useCurrentFile(pane, review, onCurrent);
  useResumeAt(current);

  return (
    <main ref={pane} className="pane overflow-y-auto bg-ground" data-current={current ?? ""}>
      {inReadingOrder(review).map((row) =>
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
            view={view}
            onReach={() => request(row.file.path)}
            onToggleViewed={() => onToggleViewed(row.file.path, !row.file.viewed)}
          />
        ),
      )}

      {review.unmapped.length > 0 && <Unmapped paths={review.unmapped} />}
    </main>
  );
}

/** The page, top to bottom: each block announced, then the files it holds, then
 * the files that belong to no block at all. */
function inReadingOrder(review: ReviewView): Row[] {
  return [
    ...review.blocks.flatMap((block, i): Row[] => [
      { block, number: i + 1 },
      ...block.files.map((file) => ({ file })),
    ]),
    ...review.looseSkim.map((file) => ({ file })),
  ];
}

type Row = { block: BlockView; number: number } | { file: FileView };

type ReadingProps = {
  review: ReviewView;
  view: DiffView;
  current: string | null;
  onCurrent: (path: string) => void;
  onToggleViewed: (path: string, viewed: boolean) => void;
  onError: (message: string) => void;
};
