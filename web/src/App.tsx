import { useCallback, useRef, useState } from "react";
import { TriangleAlert } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { TooltipProvider } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useComments } from "@/hooks/useComments";
import { useActivity } from "@/hooks/useActivity";
import { useGitHub } from "@/hooks/useGitHub";
import { useDiffView } from "@/hooks/useDiffView";
import { useOpenFiles } from "@/hooks/useOpenFiles";
import { useOpenLenses } from "@/hooks/useOpenLenses";
import { useTheme } from "@/hooks/useTheme";
import { useNavigation } from "@/hooks/useNavigation";
import { useReview } from "@/hooks/useReview";
import { useShortcuts } from "@/hooks/useShortcuts";
import { useSidebar } from "@/hooks/useSidebar";
import { useStop } from "@/hooks/useStop";
import { Activity } from "@/review/activity/Activity";
import { HelpDialog } from "@/review/HelpDialog";
import { KeyBar } from "@/review/KeyBar";
import { Reading } from "@/review/Reading";
import { Failed } from "@/review/Failed";
import { Toast } from "@/review/Toast";
import { Sidebar } from "@/review/Sidebar";
import { TopBar } from "@/review/TopBar";

/** Composition only: what is on screen and in what order. Everything that
 * decides how a thing looks lives in the piece that draws it. */
export default function App() {
  // What the reader just tried and did not get: a comment that would not save,
  // a review the host turned down. Apart from the load failure, because
  // the review is still on the screen and still worth reading, and blanking it
  // would take away the very thing the message is about.
  const [failed, setFailed] = useState<string | null>(null);
  const {
    review,
    current,
    setCurrent,
    error,
    toggleViewed,
    toggleSeen,
    stale,
    refresh,
    generation,
  } = useReview(setFailed);
  const [helpOpen, setHelpOpen] = useState(false);
  const { stopped, stop } = useStop();
  const { open: sidebarOpen, toggle, dismiss } = useSidebar();
  // Where focus goes when the sidebar it opens is taken out from under it.
  const sidebarToggle = useRef<HTMLButtonElement>(null);
  const [told, setTold] = useState<string | null>(null);
  const forget = useCallback(() => setTold(null), []);
  const [view, setView] = useDiffView();
  const files = useOpenFiles();
  const lenses = useOpenLenses();
  const [theme, setTheme] = useTheme();
  const github = useGitHub(setFailed, generation);
  const comments = useComments(setFailed, generation, github);
  const activity = useActivity(comments.all, github.pull, review?.branch ?? "", comments.loaded);
  const [activityOpen, setActivityOpen] = useState(false);

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

  const {
    order,
    index,
    goTo,
    pick,
    nextUnread,
    mark,
    markLens,
    jumpToThread,
  } = useNavigation({ review, current, setCurrent, toggleViewed, toggleSeen, files, comments });

  useShortcuts({
    review,
    order,
    index,
    current,
    goTo,
    nextUnread,
    toggleViewed: mark,
    toggleSeen: markLens,
    setHelpOpen,
    toggleSidebar,
    toggleActivity: () => setActivityOpen((open) => !open),
  });

  if (stopped) {
    return (
      <div className="stopped grid h-dvh place-items-center bg-ground">
        <div className="flex flex-col items-center gap-2 text-center">
          <h1 className="font-sans text-lg font-semibold text-ink">This review was stopped</h1>
          <p className="font-sans text-sm text-ink-muted">
            Run <code className="font-mono text-ink">farol serve</code> in the repository to open it again.
          </p>
        </div>
      </div>
    );
  }
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
          onNextUnread={nextUnread}
          github={github}
          threads={comments.all}
          onStop={stop}
          onTold={setTold}
          agent={comments.agent}
        />
        {sidebarOpen && (
          <Sidebar
            review={review}
            comments={comments.on}
            current={current}
            onPick={(place) => {
              pick(place);
              // Over the code, picking is the end of what the sidebar was for,
              // so it goes — and the row that was clicked goes with it.
              if (dismiss()) sidebarToggle.current?.focus();
            }}
            // From the mark, not the row: the reader stays where they are and
            // the item stays as open or as folded as it was.
            onCheckFile={(file) => {
              files.keep(file);
              void toggleViewed(file.path, !file.viewed);
            }}
            onCheckLens={(lens) => {
              lenses.keep(lens.key, lens.seen);
              void toggleSeen(lens, !lens.seen);
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
          lenses={lenses}
          comments={comments}
          onSeen={(lens, seen) => void markLens(lens, seen)}
        />

        <KeyBar
          theme={theme}
          onTheme={setTheme}
          sidebarOpen={sidebarOpen}
          onHelp={() => setHelpOpen(true)}
        />
        <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
        {/* Only where there is an agent to hear from, or something it already
            did: on a review with neither, the corner stays empty. */}
        {(comments.agent || activity.updates.length > 0 || activity.working.length > 0) && (
          <Activity
            activity={activity}
            open={activityOpen}
            onOpen={setActivityOpen}
            onJump={jumpToThread}
          />
        )}
        {failed && <Failed what={failed} onClose={() => setFailed(null)} />}
        {told && !failed && <Toast what={told} onGone={forget} />}
      </div>
    </TooltipProvider>
  );
}
