import type { FileView, TaggedLineNote } from "@/api";

/** What the session wrote about a span of lines, under the last line of it.
 *
 * The rail on the left is the same one the covered lines carry, so the span and
 * its prose read as one thing. */
export function LineNotes({ notes, file }: LineNotesProps) {
  return notes.map((note, i) => (
    <div
      key={i}
      className="note noted border-b border-note-rule bg-note-bg py-2 pr-6 pl-16 font-serif text-[0.9375rem] leading-relaxed text-ink-soft"
    >
      <span className="lbl mr-2 font-mono text-xs text-accent">
        {note.from}–{note.to}
        {/* Which block the note came from, and only when the file is read
            under more than one: otherwise the band above already said it. */}
        {file.tags.length > 1 ? ` · ${note.block}` : ""}
      </span>
      {note.text}
    </div>
  ));
}

type LineNotesProps = { notes: TaggedLineNote[]; file: FileView };
