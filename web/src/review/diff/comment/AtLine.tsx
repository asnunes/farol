import { Box } from "./Box";
import { Comment } from "./Comment";
import { commentsAt, numberOn } from "../line";
import type { Anchor, Commentary } from "../line";

/** Everything hanging off one row: what has already been asked there, and the
 * box if the reader is asking now.
 *
 * Not a thread — there are no replies to hang in one. A comment is a body and
 * nothing else, and two of them on the same line are two questions rather than
 * a conversation. What this gathers is what is attached to a line, which is why
 * the empty box belongs in it too.
 *
 * Both layouts show the same thing under the same line, so both draw it from
 * here rather than each assembling it from the parts. What differs is only what
 * they hand in: split view gives the row's two columns, unified gives the one
 * line standing for both sides. */
export function AtLine({ at, commentary }: AtLineProps) {
  const { path, comments, actions, select } = commentary;
  const here = commentsAt(comments, at);

  // The box sits where the finished comment will: under the last line of the
  // span, on the side it was dragged on, which is where the reader stopped
  // reading to write it.
  const span = select.composing;
  const writing = span !== null && numberOn(at[span.side], span.side) === span.to;

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
            await actions.add(path, span.side, span.from, span.to, body);
            select.close();
          }}
          onCancel={select.close}
        />
      )}
    </>
  );
}

type AtLineProps = { at: Anchor; commentary: Commentary };
