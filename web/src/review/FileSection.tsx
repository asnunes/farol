import { useEffect, useRef, useState } from "react";
import { Diff } from "@/review/diff/Diff";
import { FileHeader } from "@/review/FileHeader";
import { FileNote } from "@/review/FileNote";
import type { DiffView } from "@/hooks/useDiffView";
import type { FileDiff, FileView } from "@/api";

/** One file on the long page: its header, the prose the session wrote about it,
 * and the diff. */
export function FileSection({ file, diff, view, onReach, onToggleViewed }: FileSectionProps) {
  const heavy = file.additions + file.deletions > BIG;
  const [asked, setAsked] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  // Fetched as the reader gets near, a screenful ahead, so the code is there
  // by the time they arrive. A heavy file waits to be asked for by hand.
  useEffect(() => {
    const el = box.current;
    if (!el || (heavy && !asked)) return;

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
  }, [heavy, asked, onReach]);

  return (
    <div ref={box} className="filesection" data-path={file.path}>
      <FileHeader file={file} onToggleViewed={onToggleViewed} />

      {file.skim && file.skimReason && <FileNote>Safe to skim — {file.skimReason}</FileNote>}
      {file.notes.map((note, i) => (
        <FileNote key={i}>
          {/* Which block the note came from, and only when the file is read
              under more than one. */}
          {file.tags.length > 1 && (
            <span className="from mr-2 font-mono text-xs text-accent">{note.block}</span>
          )}
          {note.text}
        </FileNote>
      ))}

      {heavy && !asked ? (
        <div className="heavy px-6 py-6 font-sans text-sm text-muted">
          {file.additions + file.deletions} changed lines.{" "}
          <button
            className="cursor-pointer text-accent underline underline-offset-2"
            onClick={() => setAsked(true)}
          >
            Load the diff
          </button>
        </div>
      ) : diff ? (
        <Diff diff={diff} file={file} view={view} />
      ) : (
        <div className="loading px-6 py-6 font-mono text-sm text-faint">Loading diff…</div>
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
  onReach: () => void;
  onToggleViewed: () => void;
};
