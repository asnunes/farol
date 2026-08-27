import { useHighlight } from "@/hooks/useHighlight";
import { useOpened } from "@/hooks/useOpened";
import { DiffHunk } from "./DiffHunk";
import { Gap } from "./Gap";
import { Opened } from "./Opened";
import { gapsOf, opening } from "./gaps";
import { commentsOn } from "./line";
import { useLineSelection } from "./useLineSelection";
import type { Opened as OpenedRange } from "@/hooks/useOpened";
import type { Gap as GapRange, Opening } from "./gaps";
import type { CommentActions } from "@/hooks/useComments";
import type { DiffView } from "@/hooks/useDiffView";
import type { CommentView, FileDiff, FileView } from "@/api";

/** The code itself, with the session's line notes beside the lines they are
 * about and the reviewer's comments under them.
 *
 * A file arrives as the hunks git printed and the gaps between them. The gaps
 * are the part nobody asked for yet: they draw as one band with the controls
 * that open them, and what comes back sits in the reading order it has in the
 * file, above or below whatever is still closed. */
export function Diff({ diff, file, view, comments, actions }: DiffProps) {
  const tokenize = useHighlight(diff.binary ? null : diff.path);
  const { opened, open } = useOpened(file.path);

  // One selection per file: the reader writes one comment at a time, and a
  // drag started here has no business reaching into the file below.
  const select = useLineSelection();

  if (diff.binary) {
    // Nothing to read line by line, so say that rather than show an empty pane
    // the reviewer would take for a loading failure.
    return (
      <div className="diff nodiff px-6 py-6 font-sans text-sm text-ink-muted">
        Binary file — not shown
      </div>
    );
  }

  const commentary = {
    path: file.path,
    comments: commentsOn(comments, file.path),
    actions,
    select,
  };

  const gaps = gapsOf(diff);
  const last = diff.hunks.at(-1);
  const below = last && gaps.find((g) => g.from === last.newStart + last.newLines);

  const stretch = (range: OpenedRange, gap: GapRange) => (
    <Opened
      key={`open-${range.from}`}
      range={range}
      file={file}
      tokenize={tokenize}
      view={view}
      shift={gap.shift}
    />
  );

  // The stretch after the last hunk, which has no band of its own to hide in.
  const tail = below ? opening(below, opened) : null;

  return (
    <div className="diff font-mono text-[0.8125rem] leading-relaxed">
      {diff.hunks.map((hunk, i) => {
        // The gap above this hunk: the leading one for the first hunk, and the
        // one between neighbours after that.
        const gap = gaps.find((g) => g.to === hunk.newStart - 1);
        const { above, closed } = gap ? opening(gap, opened) : empty;

        // Still closed and reaching down to this hunk, so its controls belong
        // in the hunk's own band rather than in a second one right above it.
        const joined = closed !== null && closed.to === gap?.to;

        return (
          <div key={i}>
            {gap && (
              <>
                {above
                  .filter((range) => !closed || range.to < closed.from)
                  .map((range) => stretch(range, gap))}

                {closed && !joined && <Gap gap={closed} onOpen={open} />}

                {above
                  .filter((range) => closed !== null && range.from > closed.to)
                  .map((range) => stretch(range, gap))}
              </>
            )}

            <DiffHunk
              hunk={hunk}
              file={file}
              tokenize={tokenize}
              view={view}
              commentary={commentary}
              gap={joined && closed ? { gap: closed, onOpen: open } : undefined}
            />
          </div>
        );
      })}

      {below && tail && (
        <>
          {tail.above.map((range) => stretch(range, below))}
          {tail.closed && <Gap gap={tail.closed} onOpen={open} />}
        </>
      )}
    </div>
  );
}

/** A hunk with no gap above it: nothing opened, nothing left closed. */
const empty: Opening<OpenedRange> = { above: [], closed: null };

type DiffProps = {
  diff: FileDiff;
  file: FileView;
  view: DiffView;
  comments: CommentView[];
  actions: CommentActions;
};
