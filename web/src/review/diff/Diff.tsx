import { useHighlight } from "@/hooks/useHighlight";
import { DiffHunk } from "./DiffHunk";
import type { DiffView } from "@/hooks/useDiffView";
import type { FileDiff, FileView } from "@/api";

/** The code itself, with the session's line notes beside the lines they are
 * about. */
export function Diff({ diff, file, view }: DiffProps) {
  const tokenize = useHighlight(diff.binary ? null : diff.path);

  if (diff.binary) {
    // Nothing to read line by line, so say that rather than show an empty pane
    // the reviewer would take for a loading failure.
    return (
      <div className="diff nodiff px-6 py-6 font-sans text-sm text-muted">
        Binary file — not shown
      </div>
    );
  }

  return (
    <div className="diff font-mono text-[0.8125rem] leading-relaxed">
      {diff.hunks.map((hunk, i) => (
        <DiffHunk key={i} hunk={hunk} file={file} tokenize={tokenize} view={view} />
      ))}
    </div>
  );
}

type DiffProps = { diff: FileDiff; file: FileView; view: DiffView };
