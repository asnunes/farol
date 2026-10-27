import Markdown from "react-markdown";
import { cn } from "@/lib/utils";
import type { CitationView } from "@/api";

/** What a lens said, with each checked citation turned into a way to the code.
 *
 * Only the code spans the server checked become links; a citation it could
 * not check stays the code span it was written as. Inside a file's own lens a
 * citation of that same file reads "lines 90–91", because the file is the one
 * the reader is already looking at. */
export function LensText({ text, citations, inFile, onCite, className }: LensTextProps) {
  return (
    <div className={cn(MARKS, className)}>
      <Markdown
        components={{
          code: ({ children, className: language }) => {
            const said = String(children);
            const citation = !language && citations.find((c) => c.text === said);
            if (!citation) return <code>{children}</code>;
            return (
              <button
                type="button"
                className="citation inline cursor-pointer border-b border-dotted border-highlight font-mono text-[0.85em] whitespace-nowrap text-highlight"
                onClick={() => onCite(citation)}
              >
                {citation.path === inFile ? lines(citation) : said}
              </button>
            );
          },
        }}
      >
        {text}
      </Markdown>
    </div>
  );
}

function lines(citation: CitationView): string {
  const span = citation.from === citation.to ? `line ${citation.from}` : `lines ${citation.from}–${citation.to}`;
  return citation.side === "old" ? `${span} (old)` : span;
}

/** The same marks the rest of the page's prose uses. */
const MARKS =
  "[&_a]:text-highlight [&_a]:underline [&_code]:rounded [&_code]:bg-sunken [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.85em] [&_li]:ml-4 [&_li]:list-disc [&_li+li]:mt-1 [&_p+p]:mt-2 [&_pre]:mt-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-sunken [&_pre]:p-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_ul]:my-1";

type LensTextProps = {
  text: string;
  citations: CitationView[];
  /** The file this lens belongs to, when it is a file's own lens. */
  inFile?: string;
  onCite: (citation: CitationView) => void;
  className?: string;
};
