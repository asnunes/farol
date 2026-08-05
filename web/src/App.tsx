import { useCallback, useEffect, useState } from "react";
import {
  api,
  blockOf,
  readingOrder,
  type FileDiff,
  type FileView,
  type ReviewView,
} from "./api";

export default function App() {
  const [review, setReview] = useState<ReviewView | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [helpOpen, setHelpOpen] = useState(false);

  const load = useCallback(async () => {
    try {
      const next = await api.review();
      setReview(next);
      setCurrent((prev) => {
        if (prev && readingOrder(next).some((f) => f.path === prev)) return prev;
        const first = readingOrder(next).find((f) => !f.viewed) ?? readingOrder(next)[0];
        return first?.path ?? null;
      });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // The repository moving is pushed, not polled: commit on another terminal and
  // the banner appears on its own.
  useEffect(() => {
    const es = new EventSource("/api/watch");
    const refresh = () => void load();
    es.addEventListener("map", refresh);
    es.addEventListener("head", refresh);
    return () => es.close();
  }, [load]);

  useEffect(() => {
    if (!current) return;
    let cancelled = false;
    api
      .file(current)
      .then((d) => !cancelled && setDiff(d))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [current]);

  const order = review ? readingOrder(review) : [];
  const index = order.findIndex((f) => f.path === current);

  const toggleViewed = useCallback(
    async (path: string, viewed: boolean) => {
      await api.setViewed(path, viewed);
      await load();
    },
    [load],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      if (!review) return;

      const go = (i: number) => {
        const next = order[Math.max(0, Math.min(order.length - 1, i))];
        if (next) setCurrent(next.path);
      };

      switch (e.key) {
        case "j":
          go(index + 1);
          break;
        case "k":
          go(index - 1);
          break;
        case "n": {
          const next =
            order.slice(index + 1).find((f) => !f.viewed) ?? order.find((f) => !f.viewed);
          if (next) setCurrent(next.path);
          break;
        }
        case "e": {
          const file = order[index];
          if (file) void toggleViewed(file.path, !file.viewed);
          break;
        }
        case "[":
        case "]": {
          const here = blockOf(review, current ?? "");
          if (!here) break;
          const target = review.blocks[here.index + (e.key === "]" ? 1 : -1)];
          if (target?.files[0]) setCurrent(target.files[0].path);
          break;
        }
        case "?":
          setHelpOpen((v) => !v);
          break;
        case "Escape":
          setHelpOpen(false);
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [review, order, index, current, toggleViewed]);

  if (error) return <div className="fatal">{error}</div>;
  if (!review) return <div className="fatal">Loading…</div>;

  const file = order[index] ?? null;
  const here = current ? blockOf(review, current) : null;
  const done = review.totalFiles > 0 && review.viewedFiles === review.totalFiles;

  return (
    <div className="app">
      <header className="top">
        <div className="refs">
          <span className="head">{review.branch}</span>
          <span className="arrow">→</span>
          <span className="base">{review.base}</span>
        </div>
        <div className="top-right">
          {review.commitsBehind > 0 && (
            <div className="stale-chip">
              map {review.commitsBehind} commit{review.commitsBehind === 1 ? "" : "s"} behind
            </div>
          )}
          <div className="progress">
            {done ? (
              <span className="done" title="Everything read">
                🎉
              </span>
            ) : (
              <span>
                {review.viewedFiles} / {review.totalFiles}
              </span>
            )}
            <div
              className="meter"
              role="img"
              aria-label={`${review.viewedFiles} of ${review.totalFiles} read`}
            >
              <span
                style={{
                  width: `${review.totalFiles ? (review.viewedFiles / review.totalFiles) * 100 : 0}%`,
                }}
              />
            </div>
          </div>
        </div>
      </header>

      <Sidebar review={review} current={current} onPick={setCurrent} />

      <main className="pane">
        {here && (
          <div className="blockbar">
            <div className="kicker">
              block {here.index + 1} of {review.blocks.length}
            </div>
            <h2>{here.block.title}</h2>
            {here.block.context && <p>{here.block.context}</p>}
          </div>
        )}

        {file && (
          <>
            <div className="filehead">
              <div className="left">
                <button
                  className="markbox"
                  aria-pressed={file.viewed}
                  title="Mark as read — key e"
                  onClick={() => void toggleViewed(file.path, !file.viewed)}
                >
                  <span aria-hidden="true">✓</span>
                </button>
                <div className="path">
                  <span className="dir">{splitPath(file.path).dir}</span>
                  {splitPath(file.path).name}
                </div>
                {file.tags.length > 1 && (
                  <div className="tags">
                    {file.tags.map((t) => (
                      <span className="tag" key={t}>
                        {t}
                      </span>
                    ))}
                  </div>
                )}
              </div>
              <div className="churn">
                <span className="a">+{file.additions}</span>
                <span className="d">−{file.deletions}</span>
              </div>
            </div>

            {file.skim && file.skimReason && (
              <div className="filenote skim-note">
                Safe to skim — {file.skimReason}
              </div>
            )}
            {file.notes.map((n, i) => (
              <div className="filenote" key={i}>
                {file.tags.length > 1 && <span className="from">{n.block}</span>}
                {n.text}
              </div>
            ))}

            {diff && diff.path === file.path ? (
              <Diff diff={diff} file={file} />
            ) : (
              <div className="loading">Loading diff…</div>
            )}
          </>
        )}

        {review.unmapped.length > 0 && (
          <div className="unmapped">
            <strong>{review.unmapped.length} file(s) changed after this map was made</strong>
            <p>Run the review-map skill again to fold them in.</p>
            <ul>
              {review.unmapped.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          </div>
        )}
      </main>

      <nav className="keybar">
        <span>
          <kbd>j</kbd>
          <kbd>k</kbd> file
        </span>
        <span>
          <kbd>n</kbd> next unread
        </span>
        <span>
          <kbd>e</kbd> mark read
        </span>
        <span>
          <kbd>[</kbd>
          <kbd>]</kbd> block
        </span>
        <span>
          <kbd>?</kbd> help
        </span>
      </nav>

      {helpOpen && (
        <div className="help" onClick={() => setHelpOpen(false)}>
          <div className="help-card" onClick={(e) => e.stopPropagation()}>
            <h3>Keys</h3>
            <dl>
              <dt>j / k</dt>
              <dd>previous and next file, in reading order</dd>
              <dt>n</dt>
              <dd>jump to the next file you have not read</dd>
              <dt>e</dt>
              <dd>mark the current file read</dd>
              <dt>[ / ]</dt>
              <dd>previous and next block</dd>
              <dt>?</dt>
              <dd>this list</dd>
            </dl>
          </div>
        </div>
      )}
    </div>
  );
}

function Sidebar({
  review,
  current,
  onPick,
}: {
  review: ReviewView;
  current: string | null;
  onPick: (path: string) => void;
}) {
  return (
    <aside className="map">
      {review.blocks.map((block, i) => {
        const state = block.files.every((f) => f.viewed) && block.files.length > 0
          ? "done"
          : block.files.some((f) => f.path === current)
            ? "current"
            : "todo";
        return (
          <section className="blk" data-state={state} key={block.slug}>
            <div className="blk-head">
              <div className="num" aria-hidden="true">
                {state === "done" ? "✓" : i + 1}
              </div>
              <div className="blk-title">{block.title}</div>
            </div>
            <ul className="blk-files">
              {block.files.map((f) => (
                <FileRow key={f.path} file={f} current={current} onPick={onPick} />
              ))}
            </ul>
          </section>
        );
      })}

      {review.looseSkim.length > 0 && (
        <section className="blk" data-state="loose">
          <div className="blk-head">
            <div className="num loose" aria-hidden="true">
              ~
            </div>
            <div className="blk-title loose">No block · safe to skim</div>
          </div>
          <ul className="blk-files">
            {review.looseSkim.map((f) => (
              <FileRow key={f.path} file={f} current={current} onPick={onPick} />
            ))}
          </ul>
        </section>
      )}
    </aside>
  );
}

function FileRow({
  file,
  current,
  onPick,
}: {
  file: FileView;
  current: string | null;
  onPick: (path: string) => void;
}) {
  return (
    <li>
      <button
        className={`fileitem${file.skim ? " skim" : ""}`}
        data-seen={file.viewed ? "true" : undefined}
        aria-current={file.path === current ? "true" : undefined}
        title={file.skimReason ?? undefined}
        onClick={() => onPick(file.path)}
      >
        <span className="chk">{file.viewed ? "✓" : ""}</span>
        <span className="nm">{splitPath(file.path).name}</span>
        {file.skim && <span className="fast">skim</span>}
        {!file.skim && file.lineNotes.length > 0 && (
          <span className="dot" title={`${file.lineNotes.length} note(s)`}>
            {"•".repeat(Math.min(3, file.lineNotes.length))}
          </span>
        )}
      </button>
    </li>
  );
}

function Diff({ diff, file }: { diff: FileDiff; file: FileView }) {
  if (diff.binary) {
    // Nothing to read line by line, so say that rather than show an empty pane
    // the reviewer would take for a loading failure.
    return <div className="diff nodiff">Binary file — not shown</div>;
  }

  return (
    <div className="diff">
      {diff.hunks.map((hunk, hi) => (
        <div key={hi}>
          <div className="hunk">
            @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
          </div>
          {hunk.lines.map((line, li) => {
            const cls =
              line.kind === "added" ? "row add" : line.kind === "removed" ? "row del" : "row";
            const marker = line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " ";
            const notes = file.lineNotes.filter(
              (n) => line.new_number !== null && n.to === line.new_number,
            );
            return (
              <div key={li}>
                <div className={cls}>
                  <div className="ln">{line.new_number ?? line.old_number ?? ""}</div>
                  <div className="code">
                    {marker} {line.content}
                  </div>
                </div>
                {notes.map((n, ni) => (
                  <div className="note" key={ni}>
                    <span className="lbl">
                      {n.from}–{n.to}
                      {file.tags.length > 1 ? ` · ${n.block}` : ""}
                    </span>
                    {n.text}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}

/** Split once: the directory keeps its trailing slash so the two halves
 * concatenate back to the original path. */
function splitPath(path: string): { dir: string; name: string } {
  const i = path.lastIndexOf("/");
  return i === -1
    ? { dir: "", name: path }
    : { dir: path.slice(0, i + 1), name: path.slice(i + 1) };
}
