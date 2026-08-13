import { useCallback, useState } from "react";
import { readingOrder } from "@/api";
import { useComments } from "@/hooks/useComments";
import { useDiffView } from "@/hooks/useDiffView";
import { useOpenFiles } from "@/hooks/useOpenFiles";
import { useTheme } from "@/hooks/useTheme";
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
  const files = useOpenFiles();
  const [theme, setTheme] = useTheme();
  const comments = useComments(setError);

  const order = review ? readingOrder(review) : [];
  const index = order.findIndex((f) => f.path === current);

  // Moving names the file *and* scrolls to it. Naming it only through the
  // scroll would mean waiting for the observer to answer, and two presses in a
  // row would both count from the file the reader had already left.
  const goTo = useCallback(
    (path: string) => {
      setCurrent(path);
      // Going to a file opens it. Being taken to one that stayed folded away
      // because it had been read would look like arriving nowhere.
      files.set(path, true);
      scrollToFile(path);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [setCurrent],
  );

  // Marking a file read folds it away, and unmarking opens it again: the two
  // states are separate and this is where they meet. One function for both
  // ways of marking, or the keyboard and the tick box behave differently.
  const mark = useCallback(
    (path: string, viewed: boolean) => {
      files.set(path, !viewed);
      // Marking one read folds it away, which leaves the reader looking at
      // whatever was underneath. Take them to the next file instead.
      if (viewed) {
        const next = order[order.findIndex((f) => f.path === path) + 1];
        if (next) goTo(next.path);
      }
      return toggleViewed(path, viewed);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [order, toggleViewed],
  );

  useShortcuts({ review, order, index, current, goTo, toggleViewed: mark, setHelpOpen });

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
        unreadable={comments.unreadable}
      />
      <Sidebar review={review} current={current} onPick={goTo} />

      <Reading
        review={review}
        view={view}
        current={current}
        onCurrent={setCurrent}
        onToggleViewed={(path, viewed) => void mark(path, viewed)}
        onError={setError}
        files={files}
        comments={comments}
      />

      <KeyBar theme={theme} onTheme={setTheme} />
      <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
    </div>
  );
}
