import { CommentBox } from "./CommentBox";
import { LineComments } from "./LineComments";
import { commentsAt } from "./line";
import type { Commentary } from "./line";
import type { DiffLine } from "@/api";

/** Everything hanging off one line: what has already been asked there, and the
 * box if the reader is asking now.
 *
 * Both layouts show the same thing under the same line, so both draw it from
 * here rather than each assembling it from the parts. */
export function Thread({ line, commentary }: ThreadProps) {
  const { path, comments, actions, select } = commentary;
  const here = commentsAt(comments, line);

  // The box sits where the finished comment will: under the last line of the
  // span, which is where the reader stopped reading to write it.
  const span = select.composing;
  const writing = span !== null && line.new_number !== null && span.to === line.new_number;

  if (here.length === 0 && !writing) return null;

  return (
    <>
      <LineComments comments={here} actions={actions} />
      {writing && (
        <CommentBox
          span={span}
          onSave={async (body) => {
            await actions.add(path, span.from, span.to, body);
            select.close();
          }}
          onCancel={select.close}
        />
      )}
    </>
  );
}

type ThreadProps = { line: DiffLine; commentary: Commentary };
