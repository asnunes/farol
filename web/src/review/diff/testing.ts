import { vi } from "vitest";
import type { CommentActions } from "@/hooks/useComments";
import type { BlockView, CommentView, FileView, LensView, LinesAbout, ReviewView } from "@/api";

/** Comment actions that record the call and do nothing else, for the tests
 * about what reaches the screen rather than what reaches the server. */
export function noComments(): CommentActions {
  return {
    on: () => [],
    onLens: () => [],
    onBlock: () => [],
    unreadable: [],
    drifted: [],
    add: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    agent: null,
    send: vi.fn().mockResolvedValue(true),
    addAndSend: vi.fn().mockResolvedValue(undefined),
    reply: vi.fn().mockResolvedValue(undefined),
    all: [],
    pendingOffered: false,
    pendingOf: () => undefined,
    looseOn: () => [],
    writePending: vi.fn().mockResolvedValue(true),
    editPending: vi.fn().mockResolvedValue(true),
    removePending: vi.fn().mockResolvedValue(true),
    generate: vi.fn().mockResolvedValue(true),
    addAsPending: vi.fn().mockResolvedValue(true),
    loaded: true,
    isMinimized: () => false,
    toggleMinimized: vi.fn(),
    expand: vi.fn(),
  };
}

/** A thread on lines, with the span given flat the way tests think of it. */
export function comment(over: Partial<CommentView> & Partial<Omit<LinesAbout, "kind">> = {}): CommentView {
  const { path = "src/a.rs", side = "new", from = 1, to = 1, ...rest } = over;
  const lines = from === to ? `${from}` : `${from}-${to}`;
  return {
    id: "18cb-3731",
    about: { kind: "lines", path, side, from, to },
    at: `${path}:${lines}${side === "old" ? " (old)" : ""}`,
    body: "Why this order?",
    pending: null,
    replies: [],
    waiting: null,
    ...rest,
  };
}

/** One file of a review, as the screen receives it.
 *
 * Here rather than in each test file, because two of them had written the same
 * ten fields and disagreed on the two nobody was testing.
 */
export function file(over: Partial<FileView> = {}): FileView {
  return {
    path: "src/a.rs",
    status: "modified",
    additions: 1,
    deletions: 0,
    viewed: false,
    note: null,
    lineNotes: [],
    focus: null,
    lenses: [],
    folded: false,
    covered: false,
    ...over,
  };
}

/** One block of a review: its files, given as paths or as whole files. The
 * counts default to the files it renders, which is what they are whenever a
 * file belongs to one block only; a test about a block that renders nothing
 * passes its own. */
export function block(slug: string, files: Array<string | FileView> = [], over: Partial<BlockView> = {}): BlockView {
  const rendered = files.map((f) => (typeof f === "string" ? file({ path: f }) : f));
  return {
    slug,
    title: slug,
    context: "",
    lenses: [],
    files: rendered,
    totalFiles: rendered.length,
    viewedFiles: rendered.filter((f) => f.viewed).length,
    ...over,
  };
}

/** A review of these blocks, with the counts and the first unread worked out
 * from them the way the server does. */
export function review(blocks: BlockView[], over: Partial<ReviewView> = {}): ReviewView {
  const files = blocks.flatMap((b) => b.files);
  return {
    branch: "feature/x",
    base: "main",
    generatedAt: "abc1234",
    commitsBehind: 0,
    blocks,
    unmapped: [],
    totalFiles: files.length,
    viewedFiles: files.filter((f) => f.viewed).length,
    firstUnread: files.find((f) => !f.viewed)?.path ?? null,
    lenses: [],
    ...over,
  };
}

/** A written lens on the pull request; a test about another target passes the
 * target and the key the server would send with it. */
export function lens(over: Partial<LensView> = {}): LensView {
  const id = over.id ?? "overview";
  return {
    id,
    key: `pr:${id}`,
    title: id,
    state: "written",
    text: "x",
    writtenAt: "abc1234",
    changed: 0,
    citations: [],
    target: { kind: "pr" },
    seen: false,
    open: null,
    ...over,
  };
}
