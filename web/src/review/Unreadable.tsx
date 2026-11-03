import { TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { Unreadable as UnreadableView } from "@/api";

/** Comment files the store could not read.
 *
 * Up here rather than beside the code, because a comment whose header is broken
 * has no line left to sit next to — that is exactly what is wrong with it. The
 * chip names the files in its tooltip, since fixing one means opening it. */
export function Unreadable({ broken }: UnreadableProps) {
  return (
    <Tooltip delayDuration={0}>
      <TooltipTrigger asChild>
        <Badge className="unreadable cursor-default gap-1.5 rounded-full border-transparent bg-del-bg font-mono text-xs font-normal text-del-ink">
          <TriangleAlert className="size-3.5" aria-hidden="true" />
          {broken.length} comment{broken.length === 1 ? "" : "s"} unreadable
        </Badge>
      </TooltipTrigger>
      {/* A real tooltip rather than `title`, and only here: this is the one
          piece of chrome whose message is three lines per comment, which the
          browser's own tooltip crams into a strip nobody can read. */}
      <TooltipContent className="max-w-[36rem]">
        {broken.map((one) => (
          <div key={one.file} className="mb-2 last:mb-0">
            <div className="font-sans">{headline(one)}</div>
            <div className="font-sans opacity-80">{one.why}</div>
            <div className="font-mono text-[0.6875rem] opacity-70">{one.file}</div>
          </div>
        ))}
      </TooltipContent>
    </Tooltip>
  );
}

/** What the reviewer knows it by: the file it was about, or their own words. */
function headline(one: UnreadableView): string {
  return one.about ?? (one.excerpt === null ? "an empty comment" : `"${one.excerpt}"`);
}

type UnreadableProps = { broken: UnreadableView[] };
