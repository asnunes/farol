import { useHighlight } from "@/hooks/useHighlight";
import { DiffHunk } from "./DiffHunk";
import { commentsOn } from "./line";
import { useLineSelection } from "./useLineSelection";
import type { CommentActions } from "@/hooks/useComments";
import type { DiffView } from "@/hooks/useDiffView";
import type { CommentView, FileDiff, FileView } from "@/api";

/** The code itself, with the session's line notes beside the lines they are
 * about and the reviewer's comments under them. */
export function Diff({ diff, file, view, comments, actions }: DiffProps) {
  const tokenize = useHighlight(diff.binary ? null : diff.path);

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

  return (
    <div className="diff font-mono text-[0.8125rem] leading-relaxed">
      {diff.hunks.map((hunk, i) => (
        <DiffHunk
          key={i}
          hunk={hunk}
          file={file}
          tokenize={tokenize}
          view={view}
          commentary={commentary}
        />
      ))}
    </div>
  );
}

type DiffProps = {
  diff: FileDiff;
  file: FileView;
  view: DiffView;
  comments: CommentView[];
  actions: CommentActions;
};
