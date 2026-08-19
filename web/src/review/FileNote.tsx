/** Prose written by the session, in serif because that is what serif means
 * here: someone wrote this for you.
 *
 * A row rather than a line of text, so the tag naming the block it came from
 * sits beside the prose instead of inside it — the prose is markdown now, and
 * markdown starts a paragraph. */
export function FileNote({ children }: FileNoteProps) {
  return (
    <div className="filenote flex items-start gap-2 border-b border-note-rule bg-note-bg px-6 py-2.5 font-serif text-[0.9375rem] leading-relaxed text-ink-soft">
      {children}
    </div>
  );
}

type FileNoteProps = { children: React.ReactNode };
