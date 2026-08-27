import { useCallback, useState } from "react";
import { api } from "@/api";
import type { Range } from "@/review/diff/gaps";

/** The stretches of a file the reader has opened, and the text in them.
 *
 * In memory and no further. What somebody unfolded while reading is where they
 * got to rather than something they decided, and handing it back next session
 * would be deciding on their behalf that they still want it. */
export function useOpened(path: string) {
  const [opened, setOpened] = useState<Opened[]>([]);

  const open = useCallback(
    (range: Range) =>
      // The range is recorded as asked for, not as answered. The file can end
      // inside it, and then fewer lines come back and fewer rows are drawn,
      // but the gap closes either way, or the control would sit there offering
      // to fetch the same nothing again.
      api
        .lines(path, range.from, range.to)
        .then((lines) => setOpened((was) => [...was, { ...range, lines }])),
    [path],
  );

  return { opened, open };
}

export type Opened = Range & { lines: string[] };
