import { useState } from "react";
import { blockOf, readingOrder } from "@/api";
import { useDiffView } from "@/hooks/useDiffView";
import { useReview } from "@/hooks/useReview";
import { useFileDiff } from "@/hooks/useFileDiff";
import { useShortcuts } from "@/hooks/useShortcuts";
import { TopBar } from "@/review/TopBar";
import { Sidebar } from "@/review/Sidebar";
import { BlockBar } from "@/review/BlockBar";
import { FileHeader } from "@/review/FileHeader";
import { FileNote } from "@/review/FileNote";
import { Diff } from "@/review/Diff";
import { KeyBar } from "@/review/KeyBar";
import { HelpDialog } from "@/review/HelpDialog";
import { Unmapped } from "@/review/Unmapped";

/** Composition only: what is on screen and in what order. Everything that
 * decides how a thing looks lives in the piece that draws it. */
export default function App() {
  const { review, current, setCurrent, error, setError, toggleViewed } = useReview();
  const [helpOpen, setHelpOpen] = useState(false);
  const [view, setView] = useDiffView();

  const order = review ? readingOrder(review) : [];
  const index = order.findIndex((f) => f.path === current);
  const diff = useFileDiff(current, setError);

  useShortcuts({
    review,
    order,
    index,
    current,
    setCurrent,
    toggleViewed,
    setHelpOpen,
  });

  const banner = "fatal p-8 font-mono text-sm text-muted whitespace-pre-wrap";
  if (error) return <div className={banner}>{error}</div>;
  if (!review) return <div className={banner}>Loading…</div>;

  const file = order[index] ?? null;
  const here = current ? blockOf(review, current) : null;

  return (
    <div className="app grid h-screen grid-cols-[19rem_minmax(0,1fr)] grid-rows-[auto_minmax(0,1fr)]">
      <TopBar review={review} view={view} onView={setView} />
      <Sidebar review={review} current={current} onPick={setCurrent} />

      <main className="pane overflow-y-auto bg-ground">
        {here && (
          <BlockBar block={here.block} number={here.index + 1} total={review.blocks.length} />
        )}

        {file && (
          <>
            <FileHeader
              file={file}
              onToggleViewed={() => void toggleViewed(file.path, !file.viewed)}
            />

            {file.skim && file.skimReason && (
              <FileNote>Safe to skim — {file.skimReason}</FileNote>
            )}
            {file.notes.map((n, i) => (
              <FileNote key={i}>
                {file.tags.length > 1 && (
                  <span className="from mr-2 font-mono text-xs text-accent">{n.block}</span>
                )}
                {n.text}
              </FileNote>
            ))}

            {diff && diff.path === file.path ? (
              <Diff diff={diff} file={file} view={view} />
            ) : (
              <div className="loading p-8 font-mono text-sm text-muted">Loading diff…</div>
            )}
          </>
        )}

        {review.unmapped.length > 0 && <Unmapped paths={review.unmapped} />}
      </main>

      <KeyBar />
      <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
    </div>
  );
}
