import { useCallback, useRef } from "react";
import { lensAt, lensKeyOf, lensPlace, readingOrder, type CommentView, type LensView, type ReviewView } from "@/api";
import { unreadAfter } from "@/hooks/useCurrentFile";
import type { CommentActions } from "@/hooks/useComments";
import type { OpenFiles } from "@/hooks/useOpenFiles";
import { scrollToFile, scrollToLens, scrollToThread } from "@/lib/scroll";

/** How the reader moves through the review: where they are in the reading
 * order, going to a file or a lens, the next unread, and what marking
 * something read does to where they stand.
 *
 * Out of `App`, which composes the screen and should not also decide how the
 * reader moves through it; and testable on its own, where it used to be
 * reachable only through the whole page. */
export function useNavigation({ review, current, setCurrent, toggleViewed, toggleSeen, files, comments }: NavigationInputs) {
  const order = review ? readingOrder(review) : [];
  // On a lens, the keys count from just before what it opens: the block's
  // first file, or the first file of all for the pull request's.
  const onLens = review ? lensAt(review, current) : null;
  const index = onLens
    ? order.findIndex((f) => f.path === onLens.block?.files[0]?.path)
      - (onLens.block ? 1 : 0)
    : order.findIndex((f) => f.path === current);
  const reading = useRef(order);
  reading.current = order;

  // Moving names the file *and* scrolls to it. Naming it only through the
  // scroll would mean waiting for the observer to answer, and two presses in a
  // row would both count from the file the reader had already left.
  const goTo = useCallback(
    (path: string, asked = false) => {
      setCurrent(path);
      // Going to a file opens it. Being taken to one that stayed folded away
      // because it had been read would look like arriving nowhere.
      //
      // Through the ref because this function is handed to the keyboard effect
      // and has to keep its identity across renders, while the order it looks
      // the file up in is a new array on every one of them.
      //
      // Not one whose lenses stand in for it: its lenses are on screen, and
      // the code stays behind Show code unless the map puts it in focus.
      // Unless the reader asked for the file by name: then it is the code
      // they want to see.
      const file = reading.current.find((f) => f.path === path);
      if (file && (asked || !(file.folded && !file.viewed))) files.set(file, true);
      scrollToFile(path);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [setCurrent],
  );

  // Where the cursor is, is the browser's to know, so the search from it
  // happens here. Wrapping round is not: the last unread file may be behind
  // you after marking things read, and the one to wrap to is the same
  // first-unread the server hands over for the landing.
  const nextUnread = () => {
    const ahead = order.slice(index + 1).find((f) => !f.viewed)?.path;
    const next = ahead ?? review?.firstUnread;
    if (next) goTo(next);
  };

  // Marking a file read folds it away, and unmarking opens it again — both by
  // the mark itself, without a choice being recorded here. Folding ahead of the
  // server would be this side deciding a file is read before it is, and the tick
  // box and the block counts already wait for the answer; a fold that did not
  // would be the one thing on the row moving early.
  const mark = useCallback(
    (path: string, viewed: boolean) => {
      // Marking one read folds it away, which leaves the reader looking at
      // whatever was underneath. Take them to what is still to be read.
      if (viewed) goToPlace(unreadAfter(path));
      return toggleViewed(path, viewed);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [toggleViewed],
  );

  // The same for a lens: seen folds it away, so the reader is taken to what
  // comes after it rather than left on it for the next press to unfold.
  const markLens = useCallback(
    (lens: LensView, seen: boolean) => {
      // After the server answers, not before: seeing a block's lenses can
      // make its files read, and the next unread place is only known then.
      //
      // From the lens that was marked, which is not always where the reader
      // is: a tick can be clicked on a lens that has not reached the line yet.
      // A lens that is a place of its own; a file's lens sits inside its file,
      // and the reader's place stands for it.
      const at = lensPlace(lens.key);
      const from = review && lensAt(review, at) ? at : current;
      return toggleSeen(lens, seen).then(() => {
        if (seen) requestAnimationFrame(() => goToPlace(unreadAfter(from)));
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [current, review, toggleSeen],
  );

  // A place is a file path or `lens:<key>`; going to a lens names it and
  // scrolls, going to a file goes the way the keys do.
  function goToPlace(place: string | null, asked = false) {
    if (!place) return;
    const key = lensKeyOf(place);
    if (key) {
      setCurrent(place);
      scrollToLens(key);
    } else {
      goTo(place, asked);
    }
  }

  // Picked by name from the sidebar: a file the reader wants to see, even one
  // the map left folded behind its lenses.
  const pick = (place: string) => goToPlace(place, true);

  // From the activity box to a thread: its file opened if reading had folded
  // it, the thread opened if the reader had minimised it, and the thread
  // brought to the middle of the pane.
  //
  // A thread off the diff is not in its file but after its block, or at the
  // end of the page, and is always drawn: there is no file to open first.
  // A thread on a lens or a block is drawn where its text is: the lens is
  // reached the way the sidebar reaches it, a file's lens by opening its file.
  const jumpToThread = (thread: CommentView) => {
    comments.expand(thread.id);
    const about = thread.about;
    if (about.kind === "lines") {
      if (!comments.drifted.some((t) => t.id === thread.id)) goTo(about.path);
    } else if (about.kind === "lens" && about.file !== null) {
      goTo(about.file);
    }
    scrollToThread(thread.id);
  };

  return { order, index, goTo, pick, nextUnread, mark, markLens, jumpToThread };
}

export type Navigation = ReturnType<typeof useNavigation>;

type NavigationInputs = {
  review: ReviewView | null;
  /** A file path, or `lens:<key>` for a pull request or block lens. */
  current: string | null;
  setCurrent: (place: string) => void;
  toggleViewed: (path: string, viewed: boolean) => Promise<void>;
  toggleSeen: (lens: LensView, seen: boolean) => Promise<void>;
  files: OpenFiles;
  comments: Pick<CommentActions, "expand" | "drifted">;
};
