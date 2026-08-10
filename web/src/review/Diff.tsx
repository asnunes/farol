import { cn } from "@/lib/utils";
import type { FileDiff, FileView, Hunk } from "@/api";

/** The code itself, with the session's line notes beside the lines they are
 * about. */
export function Diff({ diff, file }: { diff: FileDiff; file: FileView }) {
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
    <div className="diff pb-24 font-mono text-[0.8125rem] leading-relaxed">
      {diff.hunks.map((hunk, i) => (
        <DiffHunk key={i} hunk={hunk} file={file} />
      ))}
    </div>
  );
}

function DiffHunk({ hunk, file }: { hunk: Hunk; file: FileView }) {
  return (
    <div>
      <div className="hunk bg-sunken px-6 py-1 text-xs text-faint">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>
      {hunk.lines.map((line, i) => {
        // A note is anchored to the last line of its span, which is where the
        // reader has finished reading the thing it is about.
        const notes = file.lineNotes.filter(
          (n) => line.new_number !== null && n.to === line.new_number,
        );
        const marker = line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " ";

        return (
          <div key={i}>
            <div
              className={cn(
                "row diff-row",
                line.kind === "added" && "add bg-add-bg text-add-ink",
                line.kind === "removed" && "del bg-del-bg text-del-ink",
              )}
            >
              <div className="ln shrink-0 pr-3 text-right text-faint select-none">
                {line.new_number ?? line.old_number ?? ""}
              </div>
              <div className="code overflow-x-auto whitespace-pre">
                {marker} {line.content}
              </div>
            </div>
            {notes.map((n, j) => (
              <div
                key={j}
                className="note border-y border-note-rule bg-note-bg py-2 pr-6 pl-16 font-serif text-[0.9375rem] leading-relaxed text-ink-soft"
              >
                <span className="lbl mr-2 font-mono text-xs text-accent">
                  {n.from}–{n.to}
                  {file.tags.length > 1 ? ` · ${n.block}` : ""}
                </span>
                {n.text}
              </div>
            ))}
          </div>
        );
      })}
    </div>
  );
}
