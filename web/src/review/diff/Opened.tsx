import { useMemo } from "react";
import { DiffWindow } from "./DiffWindow";
import { WINDOW_LINES } from "./hunkWindows";
import { useHighlight } from "@/hooks/useHighlight";
import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { notedBy, notesAt } from "./line";
import type { Opened as Range } from "@/hooks/useOpened";
import type { DiffView } from "@/hooks/useDiffView";
import type { DiffLine, FileView } from "@/api";

/** Lines the diff never printed, opened by the reader.
 *
 * They take no comment. GitHub's rule is that a review comment sits on a line
 * the diff reaches, and farol refuses one at writing time rather than letting
 * the host refuse it at sending time — so the gutter here is a number and
 * nothing else. Passing no commentary is what says so: the gutter draws no `+`
 * and starts no drag without one.
 *
 * The session's line notes do show. A note can be pinned anywhere in the file,
 * and one pinned outside the diff has been written and invisible until now. */
export function Opened({ range, file, view, shift }: OpenedProps) {
  const chunks = Array.from(
    { length: Math.ceil(range.lines.length / WINDOW_LINES) },
    (_, i) => i * WINDOW_LINES,
  );
  return chunks.map((from) => (
    <DiffWindow
      key={from}
      rows={Math.min(WINDOW_LINES, range.lines.length - from)}
      layout={view}
    >
      <OpenedChunk
        range={range}
        file={file}
        view={view}
        shift={shift}
        from={from}
      />
    </DiffWindow>
  ));
}

function OpenedChunk({
  range,
  file,
  view,
  shift,
  from,
}: OpenedProps & { from: number }) {
  const tokenize = useHighlight(file.path);
  const to = Math.min(from + WINDOW_LINES, range.lines.length);
  const coloured = useMemo(
    // A whole stretch at a time, not line by line: a tokenizer carries state
    // across lines, and a block comment read one line at a time comes out as
    // code.
    () =>
      !tokenize
        ? null
        : tokenize.range
          ? tokenize.range(range.lines, from, to)
          : tokenize(range.lines.slice(0, to).join("\n")).slice(from, to),
    [range.lines, tokenize, from, to],
  );

  return range.lines.slice(from, to).map((content, i) => {
    const line = at(range.from + from + i, shift, content);
    const marked = notedBy(file, line) && "noted";

    return (
      <div key={line.newNumber}>
        {view === "split" ? (
          <div className={cn("row split-row bg-surface", marked)}>
            <LineNumber line={line} side="old" />
            <div className="code">
              <Code tokens={coloured?.[i]} plain={content} />
            </div>
            <LineNumber line={line} side="new" />
            <div className="code">
              <Code tokens={coloured?.[i]} plain={content} />
            </div>
          </div>
        ) : (
          <div className={cn("row diff-row bg-surface", marked)}>
            <LineNumber line={line} side="new" />
            <div className="code">
              {"  "}
              <Code tokens={coloured?.[i]} plain={content} />
            </div>
          </div>
        )}

        <LineNotes notes={notesAt(file, line)} file={file} />
      </div>
    );
  });
}

/** One opened line, in the shape the rest of the diff is written against. Every
 * line here is context by definition, and its number on the old side is the new
 * one plus the shift the surrounding hunks establish. */
function at(number: number, shift: number, content: string): DiffLine {
  return {
    kind: "context",
    oldNumber: number + shift,
    newNumber: number,
    content,
  };
}

type OpenedProps = {
  range: Range;
  file: FileView;
  view: DiffView;
  /** Old number minus new number, constant across the gap this came from. */
  shift: number;
};
