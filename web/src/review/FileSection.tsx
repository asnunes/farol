import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Diff } from "@/review/diff/Diff";
import { FileHeader } from "@/review/FileHeader";
import { FileNote } from "@/review/FileNote";
import { Prose } from "@/review/Prose";
import type { CommentActions } from "@/hooks/useComments";
import type { DiffView } from "@/hooks/useDiffView";
import type { CommentView, FileDiff, FileView } from "@/api";

/** One file on the long page: its header, the prose the session wrote about it,
 * and the diff. */
export function FileSection({
  file,
  diff,
  view,
  open,
  onToggleOpen,
  onReach,
  onToggleViewed,
  comments,
  commentActions,
}: FileSectionProps) {
  const heavy = file.additions + file.deletions > BIG;
  const [asked, setAsked] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  // Fetched as the reader gets near, a screenful ahead, so the code is there
  // by the time they arrive. A heavy file waits to be asked for by hand.
  useEffect(() => {
    const el = box.current;
    // A closed file is not worth fetching: it has been read, or the reader
    // folded it away.
    if (!el || !open || (heavy && !asked)) return;

    // No observer means no reason to hold anything back: it is a browser too
    // old to be running this, or a test.
    if (typeof IntersectionObserver === "undefined") {
      onReach();
      return;
    }

    const watching = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        watching.disconnect();
        onReach();
      },
      { root: el.closest(".pane"), rootMargin: "800px 0px" },
    );
    watching.observe(el);
    return () => watching.disconnect();
  }, [open, heavy, asked, onReach]);

  return (
    <div ref={box} className="filesection" data-path={file.path}>
      <FileHeader
        file={file}
        open={open}
        onToggleOpen={onToggleOpen}
        onToggleViewed={onToggleViewed}
        comments={comments}
      />

      {!open ? null : (
        <>
          {file.skim && file.skimReason && <FileNote>Safe to skim — {file.skimReason}</FileNote>}
          {file.notes.map((note, i) => (
            <FileNote key={i}>
              {/* Which block the note came from, and only when the file is read
                  under more than one. */}
              {file.tags.length > 1 && (
                <span className="from mt-1 shrink-0 font-mono text-xs text-accent">
                  {note.block}
                </span>
              )}
              <Prose className="min-w-0 flex-1">{note.text}</Prose>
            </FileNote>
          ))}

          {heavy && !asked ? (
            <div className="heavy px-6 py-6 font-sans text-sm text-muted">
              {file.additions + file.deletions} changed lines.{" "}
              <Button
                variant="link"
                size="xs"
                className="cursor-pointer px-0 text-accent"
                onClick={() => setAsked(true)}
              >
                Load the diff
              </Button>
            </div>
          ) : diff ? (
            <Diff diff={diff} file={file} view={view} comments={comments} actions={commentActions} />
          ) : (
            <div className="loading px-6 py-6 font-mono text-sm text-faint">Loading diff…</div>
          )}
        </>
      )}
    </div>
  );
}

/** Past this many changed lines a file is not drawn until it is asked for.
 *
 * Not a memory limit but a reading one: nobody scrolls through two thousand
 * lines of a generated file, and drawing them costs everyone below it. */
const BIG = 500;

type FileSectionProps = {
  file: FileView;
  diff?: FileDiff;
  view: DiffView;
  open: boolean;
  onToggleOpen: () => void;
  onReach: () => void;
  onToggleViewed: () => void;
  /** Every comment in the review; the diff picks out this file's own. */
  comments: CommentView[];
  commentActions: CommentActions;
};
