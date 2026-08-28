import Markdown from "react-markdown";
import { cn } from "@/lib/utils";

/** Markdown written by a person, in the few marks a review actually uses.
 *
 * A typography reset is not in play: these are a sentence or two, and `prose`
 * would give a lone paragraph margins it does not need. Font, size and colour
 * come from whoever places it — serif is the session explaining, sans is the
 * reader asking — so this chooses neither, and the code spans size themselves
 * against whatever they land in. */
export function Prose({ children, className }: ProseProps) {
  return (
    <div className={cn(MARKS, className)}>
      <Markdown>{children}</Markdown>
    </div>
  );
}

/** Only what turns up in a review: a backticked identifier, an emphasis, a
 * link, the odd list, the rare block of code. */
const MARKS =
  "[&_a]:text-highlight [&_a]:underline [&_code]:rounded [&_code]:bg-sunken [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.9em] [&_li]:ml-4 [&_li]:list-disc [&_li+li]:mt-1 [&_p+p]:mt-2 [&_pre]:mt-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-sunken [&_pre]:p-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_ul]:my-2";

type ProseProps = { children: string; className?: string };
