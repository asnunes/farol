import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { notedBy, notesAt } from "./line";
import type { Opened as Range } from "@/hooks/useOpened";
import type { Tokenize } from "@/highlight/tokens";
import type { DiffView } from "@/hooks/useDiffView";
import type { DiffLine, FileView } from "@/api";

/** Lines the diff never printed, opened by the reader.
 *
 * They take no comment. GitHub's rule is that a review comment sits on a line
 * the diff reaches, and farol refuses one at writing time rather than letting
 * the host refuse it at sending time — so the gutter here is a number and
 * nothing else, the same as a removed line's. Passing no commentary is what
 * says so: the gutter draws no `+` and starts no drag without one.
 *
 * The session's line notes do show. A note can be pinned anywhere in the file,
 * and one pinned outside the diff has been written and invisible until now. */
export function Opened({ range, file, tokenize, view, shift }: OpenedProps) {
  const coloured = useMemo(
    // A whole stretch at a time, not line by line: a tokenizer carries state
    // across lines, and a block comment read one line at a time comes out as
    // code.
    () => (tokenize ? tokenize(range.lines.join("\n")) : null),
    [range.lines, tokenize],
  );

  return range.lines.map((content, i) => {
    const line = at(range.from + i, shift, content);
    const marked = notedBy(file, line) && "noted";

    return (
      <div key={line.new_number}>
        {view === "split" ? (
          <div className={cn("row split-row bg-surface", marked)}>
            <LineNumber number={line.old_number} on={null} />
            <div className="code break-words whitespace-pre-wrap">
              <Code tokens={coloured?.[i]} plain={content} />
            </div>
            <LineNumber number={line.new_number} on={null} />
            <div className="code break-words whitespace-pre-wrap">
              <Code tokens={coloured?.[i]} plain={content} />
            </div>
          </div>
        ) : (
          <div className={cn("row diff-row bg-surface", marked)}>
            <LineNumber number={line.new_number} on={null} />
            <div className="code break-words whitespace-pre-wrap">
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
    old_number: number + shift,
    new_number: number,
    content,
  };
}

type OpenedProps = {
  range: Range;
  file: FileView;
  tokenize: Tokenize | null;
  view: DiffView;
  /** Old number minus new number, constant across the gap this came from. */
  shift: number;
};
