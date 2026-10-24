import type { CommentView, RemotePendingView } from "@/api";

/** Where the thread is on its way to the pull request: a conversation, a
 * pending comment, or a published one. */
export function Stage({ thread, remote }: StageProps) {
  const now =
    thread.pending === null ? "thread" : remote?.state === "published" ? "published" : "pending";
  return (
    <p className="stage font-sans text-[0.6875rem] tracking-wide text-faint">
      <span className={now === "thread" ? "font-semibold text-ink-soft" : undefined}>Thread</span>
      {" · "}
      <span className={now === "pending" ? "font-semibold text-ink-soft" : undefined}>Pending</span>
      {" · "}
      <span className={now === "published" ? "font-semibold text-add-ink" : undefined}>Published</span>
    </p>
  );
}

type StageProps = { thread: CommentView; remote: RemotePendingView | undefined };
