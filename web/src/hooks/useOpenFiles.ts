import { useState } from "react";
import type { FileView } from "@/api";

/** Which files are open on the page.
 *
 * A file that has been read is closed: it is done, and the ones still to read
 * should not be buried under it. So is one the lenses already speak for, as it
 * stands: the reader learns it from them, and opens the code when they want it. Anything the reader says overrides that — a
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
      return said?.against === standing(file) ? said.open : !file.viewed && !file.folded;
    },
    set: (file: FileView, open: boolean) =>
      setChoice((all) => ({ ...all, [file.path]: { open, against: standing(file) } })),
  };
}

export type OpenFiles = ReturnType<typeof useOpenFiles>;

/** What decides a file's opening on its own: whether it was read, and whether
 * the lenses speak for it as it is. A choice the reader made lapses when
 * either changes, so a file that changed under its lenses comes back open. */
function standing(file: FileView): string {
  return `${file.viewed}:${file.folded}`;
}

/** What the reader asked for, and the standing it was asking against. */
type Choice = { open: boolean; against: string };
