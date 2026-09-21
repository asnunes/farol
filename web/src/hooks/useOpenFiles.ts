import { useState } from "react";
import type { FileView } from "@/api";

/** Which files are open on the page.
 *
 * A file that has been read is closed: it is done, and the ones still to read
 * should not be buried under it. Anything the reader says overrides that — a
 * file they opened stays open however it is marked — but only for as long as
 * the thing it was overriding holds.
 *
 * That last part is the whole of this hook. An override used to be kept
 * forever, so a file folded away because it had been read stayed folded after
 * it stopped being read: the file changed underneath, the server took the tick
 * off it, and it sat there minimised with nothing to say it wanted reading
 * again. Remembering what each choice was made against lets it lapse on its own
 * when the answer it was arguing with changes. */
export function useOpenFiles() {
  const [choice, setChoice] = useState<Record<string, Choice>>({});

  return {
    isOpen: (file: FileView) => {
      const said = choice[file.path];
      return said?.against === file.viewed ? said.open : !file.viewed;
    },
    set: (file: FileView, open: boolean) =>
      setChoice((all) => ({ ...all, [file.path]: { open, against: file.viewed } })),
  };
}

export type OpenFiles = ReturnType<typeof useOpenFiles>;

/** What the reader asked for, and the read mark it was asking against. */
type Choice = { open: boolean; against: boolean };
