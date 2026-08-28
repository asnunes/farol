import { Box } from "./Box";
import { Comment } from "./Comment";
import { commentsAt } from "../line";
import type { Commentary } from "../line";
import type { DiffLine } from "@/api";

/** Everything hanging off one line: what has already been asked there, and the
 * box if the reader is asking now.
 *
 * Not a thread — there are no replies to hang in one. A comment is a body and
 * nothing else, and two of them on the same line are two questions rather than
 * a conversation. What this gathers is what is attached to a line, which is why
 * the empty box belongs in it too.
 *
 * Both layouts show the same thing under the same line, so both draw it from
 * here rather than each assembling it from the parts. */
export function AtLine({ line, commentary }: AtLineProps) {
  const { path, comments, actions, select } = commentary;
  const here = commentsAt(comments, line);

  // The box sits where the finished comment will: under the last line of the
  // span, which is where the reader stopped reading to write it.
  const span = select.composing;
  const writing = span !== null && line.newNumber !== null && span.to === line.newNumber;

  if (here.length === 0 && !writing) return null;

  return (
    <>
      {here.map((comment) => (
        <Comment key={comment.id} comment={comment} actions={actions} />
      ))}
      {writing && (
        <Box
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

type AtLineProps = { line: DiffLine; commentary: Commentary };
