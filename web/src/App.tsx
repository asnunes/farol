import { useCallback, useState } from "react";
import { readingOrder } from "@/api";
import { useDiffView } from "@/hooks/useDiffView";
import { useReview } from "@/hooks/useReview";
import { useShortcuts } from "@/hooks/useShortcuts";
import { HelpDialog } from "@/review/HelpDialog";
import { KeyBar } from "@/review/KeyBar";
import { scrollToFile } from "@/lib/scroll";
import { Reading } from "@/review/Reading";
import { Sidebar } from "@/review/Sidebar";
import { TopBar } from "@/review/TopBar";

/** Composition only: what is on screen and in what order. Everything that
 * decides how a thing looks lives in the piece that draws it. */
export default function App() {
  const { review, current, setCurrent, error, setError, toggleViewed, stale, refresh } =
    useReview();
  const [helpOpen, setHelpOpen] = useState(false);
  const [view, setView] = useDiffView();

  const order = review ? readingOrder(review) : [];
  const index = order.findIndex((f) => f.path === current);

  // Moving names the file *and* scrolls to it. Naming it only through the
  // scroll would mean waiting for the observer to answer, and two presses in a
  // row would both count from the file the reader had already left.
  const goTo = useCallback(
    (path: string) => {
      setCurrent(path);
      scrollToFile(path);
    },
    [setCurrent],
  );

  useShortcuts({ review, order, index, current, goTo, toggleViewed, setHelpOpen });

  const banner = "fatal p-8 font-mono text-sm text-muted whitespace-pre-wrap";
  if (error) return <div className={banner}>{error}</div>;
  if (!review) return <div className={banner}>Loading…</div>;

  return (
    <div className="app grid h-screen grid-cols-[19rem_minmax(0,1fr)] grid-rows-[auto_minmax(0,1fr)]">
      <TopBar
        review={review}
        view={view}
        onView={setView}
        stale={stale}
        onRefresh={() => void refresh()}
      />
      <Sidebar review={review} current={current} onPick={goTo} />

      <Reading
        review={review}
        view={view}
        current={current}
        onCurrent={setCurrent}
        onToggleViewed={(path, viewed) => void toggleViewed(path, viewed)}
        onError={setError}
      />

      <KeyBar />
      <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
    </div>
  );
}
