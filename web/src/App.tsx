import { useCallback, useRef, useState } from "react";
import { TriangleAlert, X } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { readingOrder } from "@/api";
import { cn } from "@/lib/utils";
import { useComments } from "@/hooks/useComments";
import { useDiffView } from "@/hooks/useDiffView";
import { useOpenFiles } from "@/hooks/useOpenFiles";
import { useTheme } from "@/hooks/useTheme";
import { useReview } from "@/hooks/useReview";
import { useShortcuts } from "@/hooks/useShortcuts";
import { useSidebar } from "@/hooks/useSidebar";
import { HelpDialog } from "@/review/HelpDialog";
import { KeyBar } from "@/review/KeyBar";
import { scrollToFile } from "@/lib/scroll";
import { Reading } from "@/review/Reading";
import { Sidebar } from "@/review/Sidebar";
import { TopBar } from "@/review/TopBar";

/** Composition only: what is on screen and in what order. Everything that
 * decides how a thing looks lives in the piece that draws it. */
export default function App() {
  const {
    review,
    current,
    setCurrent,
    error,
    toggleViewed,
    stale,
    refresh,
    generation,
  } = useReview();
  const [helpOpen, setHelpOpen] = useState(false);
  const { open: sidebarOpen, toggle, dismiss } = useSidebar();
  // Where focus goes when the sidebar it opens is taken out from under it.
  const sidebarToggle = useRef<HTMLButtonElement>(null);
  // What the reader just tried and did not get: a comment that would not save,
  // a review the host turned down. Apart from the load failure above, because
  // the review is still on the screen and still worth reading, and blanking it
  // would take away the very thing the message is about.
  const [failed, setFailed] = useState<string | null>(null);
  const [view, setView] = useDiffView();
  const files = useOpenFiles();
  const [theme, setTheme] = useTheme();
  const comments = useComments(setFailed, generation);

  // The sidebar can go while the reader is standing in it, and focus has to be
  // put somewhere before it does — dropped on the document, the next Tab
  // starts the page over. Only from inside it: ⌘B is deliberately live while a
  // comment is being typed, and that box keeps the cursor.
  const toggleSidebar = useCallback(() => {
    if (document.querySelector(".sidebar")?.contains(document.activeElement)) {
      sidebarToggle.current?.focus();
    }
    toggle();
  }, [toggle]);

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

  useShortcuts({
    review,
    order,
    index,
    current,
    goTo,
    toggleViewed: mark,
    setHelpOpen,
    toggleSidebar,
  });

  if (error) {
    return (
      <Alert
        variant="destructive"
        className="fatal m-4 w-auto border-rule bg-surface md:m-8"
      >
        <TriangleAlert />
        <AlertTitle className="font-sans">
          The review could not be loaded
        </AlertTitle>
        <AlertDescription className="font-mono text-sm whitespace-pre-wrap">
          {error}
        </AlertDescription>
      </Alert>
    );
  }
  if (!review) {
    return (
      <div className="fatal p-8 font-mono text-sm text-ink-muted">Loading…</div>
    );
  }

  return (
    <TooltipProvider>
      <div
        className={cn(
          // `dvh` and not `vh`: a phone's browser chrome slides in and out over
          // the bottom of the viewport, and `vh` measures it as if it were
          // never there — which puts the key bar under it.
          "app grid h-dvh",
          sidebarOpen
            ? "grid-cols-[minmax(0,1fr)] grid-rows-[auto_auto_minmax(0,1fr)] md:grid-cols-[19rem_minmax(0,1fr)] md:grid-rows-[auto_minmax(0,1fr)]"
            : "grid-cols-[minmax(0,1fr)] grid-rows-[auto_minmax(0,1fr)]",
        )}
      >
        <TopBar
          review={review}
          view={view}
          onView={setView}
          stale={stale}
          onRefresh={() => void refresh()}
          unreadable={comments.unreadable}
          sidebarOpen={sidebarOpen}
          onToggleSidebar={toggleSidebar}
          toggleRef={sidebarToggle}
        />
        {sidebarOpen && (
          <Sidebar
            review={review}
            comments={comments.comments}
            current={current}
            onPick={(path) => {
              goTo(path);
              // Over the code, picking is the end of what the sidebar was for,
              // so it goes — and the row that was clicked goes with it.
              if (dismiss()) sidebarToggle.current?.focus();
            }}
          />
        )}

        <Reading
          key={generation}
          review={review}
          view={view}
          current={current}
          onCurrent={setCurrent}
          onToggleViewed={(path, viewed) => void mark(path, viewed)}
          files={files}
          comments={comments}
        />

        <KeyBar
          theme={theme}
          onTheme={setTheme}
          sidebarOpen={sidebarOpen}
          onHelp={() => setHelpOpen(true)}
        />
        <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
        {failed && <Failed what={failed} onClose={() => setFailed(null)} />}
      </div>
    </TooltipProvider>
  );
}

/** Something the reader tried that did not happen.
 *
 * Over the review rather than in it: the page keeps its shape, nothing below
 * jumps, and what they were reading when it failed is still where they left it.
 * It sits above the key bar and waits to be dismissed, because a message that
 * fades on its own is one they can miss while looking at the code. */
function Failed({ what, onClose }: { what: string; onClose: () => void }) {
  return (
    <Alert
      variant="destructive"
      role="alert"
      className="failed fixed right-4 bottom-14 left-4 z-50 flex w-auto max-w-[34rem] items-start gap-3 border-rule bg-surface shadow-lg sm:left-auto"
    >
      <TriangleAlert />
      <AlertDescription className="min-w-0 flex-1 font-mono text-sm whitespace-pre-wrap">
        {what}
      </AlertDescription>
      <Button
        size="icon-xs"
        variant="ghost"
        className="text-faint hover:text-ink"
        aria-label="Dismiss"
        onClick={onClose}
      >
        <X className="size-3.5" aria-hidden="true" />
      </Button>
    </Alert>
  );
}
