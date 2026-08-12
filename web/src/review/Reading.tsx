import { useCallback, useEffect, useRef } from "react";
import { readingOrder } from "@/api";
import { useDiffs } from "@/hooks/useDiffs";
import { BlockBar } from "@/review/BlockBar";
import { FileSection } from "@/review/FileSection";
import { Unmapped } from "@/review/Unmapped";
import type { DiffView } from "@/hooks/useDiffView";
import type { ReviewView } from "@/api";

/** The whole review, in one scroll.
 *
 * Every block and every file is on the page in reading order, the way the
 * session laid them out, rather than one file at a time. Which file the reader
 * is on is read off the scroll instead of being chosen: the keys and the
 * sidebar scroll, and the page reports back what arrived. */
export function Reading({ review, view, current, onCurrent, onToggleViewed, onError }: ReadingProps) {
  const pane = useRef<HTMLElement>(null);
  const { diffs, request } = useDiffs(onError);

  useReportsWhatIsOnScreen(pane, review, onCurrent);
  useLandsWhereTheReaderLeftOff(current);

  return (
    <main ref={pane} className="pane overflow-y-auto bg-ground" data-current={current ?? ""}>
      {review.blocks.map((block, i) => (
        <section key={block.slug} className="blocksection" data-block={block.slug}>
          <BlockBar block={block} number={i + 1} total={review.blocks.length} />
          {block.files.map((file) => (
            <FileSection
              key={file.path}
              file={file}
              diff={diffs[file.path]}
              view={view}
              onReach={() => request(file.path)}
              onToggleViewed={() => onToggleViewed(file.path, !file.viewed)}
            />
          ))}
        </section>
      ))}

      {review.looseSkim.map((file) => (
        <FileSection
          key={file.path}
          file={file}
          diff={diffs[file.path]}
          view={view}
          onReach={() => request(file.path)}
          onToggleViewed={() => onToggleViewed(file.path, !file.viewed)}
        />
      ))}

      {review.unmapped.length > 0 && <Unmapped paths={review.unmapped} />}
    </main>
  );
}

/** Bring a file to the top of the pane. The keys and the sidebar go through
 * here rather than setting the current file themselves, so what the reader is
 * on always follows what is actually on screen. */
export function scrollToFile(path: string) {
  document.querySelector(`[data-path="${CSS.escape(path)}"]`)?.scrollIntoView({ block: "start" });
}

/** Take the reader to where they stopped, once, when the review arrives.
 *
 * The page opens at the top otherwise, and the first unread file is halfway
 * down a review that has been read before. */
function useLandsWhereTheReaderLeftOff(current: string | null) {
  const landed = useRef(false);

  useEffect(() => {
    if (landed.current || !current) return;
    landed.current = true;
    scrollToFile(current);
  }, [current]);
}

/** Name the file the reader is on, from where the pane is scrolled.
 *
 * The file whose header last passed a line a quarter down the pane, which is
 * the one being worked on. Two ends need saying out loud: at the very top no
 * header has passed the line yet, and at the very bottom the last files never
 * reach it at all, because the scroll runs out first. Reading only the top slice
 * left the last file of a review impossible to mark with the keyboard, since it
 * never became the current one.
 *
 * Measured on scroll rather than watched with an observer: an observer reports
 * crossings, and neither end of the list produces one. */
function useReportsWhatIsOnScreen(
  pane: React.RefObject<HTMLElement | null>,
  review: ReviewView,
  onCurrent: (path: string) => void,
) {
  const onCurrentRef = useRef(onCurrent);
  onCurrentRef.current = onCurrent;

  useEffect(() => {
    const root = pane.current;
    if (!root) return;

    let queued = 0;
    const look = () => {
      const sections = [...root.querySelectorAll<HTMLElement>("[data-path]")];
      if (!sections.length) return;

      const pane = root.getBoundingClientRect();
      const atTheEnd = root.scrollTop + root.clientHeight >= root.scrollHeight - 2;
      const line = pane.top + pane.height * READING_LINE;

      const visible = sections.filter(
        (el) => el.getBoundingClientRect().bottom > pane.top && el.getBoundingClientRect().top < pane.bottom,
      );
      if (!visible.length) return;

      const passed = visible.filter((el) => el.getBoundingClientRect().top <= line);
      const here = atTheEnd ? visible[visible.length - 1] : (passed[passed.length - 1] ?? visible[0]);

      if (here.dataset.path) onCurrentRef.current(here.dataset.path);
    };

    const onScroll = () => {
      cancelAnimationFrame(queued);
      queued = requestAnimationFrame(look);
    };

    look();
    root.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      cancelAnimationFrame(queued);
      root.removeEventListener("scroll", onScroll);
    };
  }, [pane, review]);
}

/** How far down the pane a header has to be before the file counts as the one
 * being read. Near the top, because the reader works downwards. */
const READING_LINE = 0.25;

type ReadingProps = {
  review: ReviewView;
  view: DiffView;
  current: string | null;
  onCurrent: (path: string) => void;
  onToggleViewed: (path: string, viewed: boolean) => void;
  onError: (message: string) => void;
};
