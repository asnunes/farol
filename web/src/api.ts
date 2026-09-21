/** Everything the browser asks the server for. */
export const api = {
  review: () => fetch("/api/review").then(json<ReviewView>),
  file: (path: string) =>
    fetch(`/api/file?path=${encodeURIComponent(path)}`).then(json<FileDiff>),
  setViewed: (path: string, viewed: boolean) =>
    send("/api/viewed", "POST", { path, viewed }),

  /** A stretch of the file the diff did not print, for a reader opening a gap. */
  lines: (path: string, from: number, to: number) =>
    fetch(`/api/lines?path=${encodeURIComponent(path)}&from=${from}&to=${to}`)
      .then(json<{ lines: string[] }>)
      .then((answer) => answer.lines),

  comments: () => fetch("/api/comments").then(json<CommentsView>),
  addComment: (path: string, side: Side, from: number, to: number, body: string) =>
    send("/api/comments", "POST", { path, side, from, to, body }),
  closeComment: (id: string) => send(`/api/comments/${encodeURIComponent(id)}`, "DELETE"),
};

/** Flat reading order across blocks — what j/k and "next unread" walk. */
export function readingOrder(review: ReviewView): FileView[] {
  return [...review.blocks.flatMap((b) => b.files), ...review.looseSkim];
}

/** The block a file is rendered under, for the band above the diff. */
export function blockOf(review: ReviewView, path: string): FileHome | null {
  for (let i = 0; i < review.blocks.length; i++) {
    if (review.blocks[i].files.some((f) => f.path === path)) {
      return { block: review.blocks[i], index: i };
    }
  }
  return null;
}

/** The whole review, as one answer. Read this first: the shapes below are the
 * pieces it is built from, and this is the one the screen is drawn against. */
export type ReviewView = {
  branch: string;
  base: string;
  generatedAt: string;
  commitsBehind: number;
  blocks: BlockView[];
  looseSkim: FileView[];
  unmapped: string[];
  totalFiles: number;
  viewedFiles: number;
  /** The first file in reading order nobody has read yet, and null once they
   * all have been. Where the reader is put when the map arrives, and where the
   * key that walks to the next unread file wraps round to. */
  firstUnread: string | null;
};

export type BlockView = {
  slug: string;
  title: string;
  context: string;
  files: FileView[];
  /** What the block holds, which is more than it renders: a file it shares with
   * an earlier block is read there and listed in neither list. Counted by the
   * server, which is the only side that knows the membership. Never zero. */
  totalFiles: number;
  viewedFiles: number;
};

export type FileView = {
  path: string;
  status: string;
  additions: number;
  deletions: number;
  viewed: boolean;
  notes: TaggedNote[];
  lineNotes: TaggedLineNote[];
  tags: string[];
  skim: boolean;
  skimReason: string | null;
};

export type TaggedNote = { block: string; text: string };

export type TaggedLineNote = {
  block: string;
  from: number;
  to: number;
  text: string;
};

/** One file's diff, fetched when the reader opens it. */
export type FileDiff = {
  path: string;
  status: string;
  hunks: Hunk[];
  /** git will not diff this file: binary content, or `-diff` in .gitattributes. */
  binary: boolean;
  additions: number;
  deletions: number;
  /** How long the file is after the change, which is where the last gap ends. */
  lineCount: number;
};

export type Hunk = {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
};

/** Which side of the diff a line number is counted on.
 *
 * Both sides number from one, so the same number names two different lines —
 * the file as it was, and the file as it now reads. Anything anchored to a line
 * carries this with it, all the way to the host, where it is spelled LEFT and
 * RIGHT. */
export type Side = "old" | "new";

export type DiffLine = {
  kind: "context" | "added" | "removed";
  oldNumber: number | null;
  newNumber: number | null;
  content: string;
};

/** The comments, and the files the store could not read.
 *
 * The unreadable ones travel with the list because the page is the only place
 * their absence shows: a comment whose markdown got broken by hand stops
 * rendering, and silence there reads as never having written it. */
export type CommentsView = {
  /** The comments of one file, under its path, oldest first. Shaped by the
   * server like every other answer, so the browser looks a file up instead of
   * grouping the list again wherever it needs the cut. */
  files: Record<string, CommentView[]>;
  unreadable: Unreadable[];
};

/** A comment the store could not read.
 *
 * What broke is the header, which is the part saying where the comment belongs
 * — so `about` is there only when the header still names a file. When it does
 * not, `excerpt` is what the reviewer recognises it by. */
export type Unreadable = {
  /** The file to open to fix it, from the root of the worktree. */
  file: string;
  about: string | null;
  excerpt: string | null;
  why: string;
};

/** A question the reviewer left over a span of lines they were reading.
 *
 * There is no answered state: closing one removes it, so every comment the
 * page holds is still waiting. */
export type CommentView = {
  id: string;
  path: string;
  /** Which side of the diff `from` and `to` are counted on. */
  side: Side;
  from: number;
  to: number;
  /** Markdown, as it was typed. */
  body: string;
  /** Where it can be read on the pull request, once it has gone. Publishing is
   * not answering: a published comment is still waiting for one. */
  published: string | null;
};

/** Where a file sits: the block it is read under and how far down the map that
 * block is, which is what the band above the diff counts off. */
export type FileHome = { block: BlockView; index: number };

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(await res.text());
  return res.json() as Promise<T>;
}


async function send(url: string, method: string, body?: unknown) {
  const res = await fetch(url, {
    method,
    ...(body === undefined
      ? {}
      : { headers: { "content-type": "application/json" }, body: JSON.stringify(body) }),
  });
  if (!res.ok) throw new Error(await res.text());
}
