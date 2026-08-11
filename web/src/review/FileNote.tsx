/** Prose written by the session, in serif because that is what serif means
 * here: someone wrote this for you. */
export function FileNote({ children }: FileNoteProps) {
  return (
    <div className="filenote border-b border-note-rule bg-note-bg px-6 py-2.5 font-serif text-[0.9375rem] leading-relaxed text-ink-soft">
      {children}
    </div>
  );
}

type FileNoteProps = { children: React.ReactNode };
