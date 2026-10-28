import { vi } from "vitest";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView, FileView } from "@/api";

/** Comment actions that record the call and do nothing else, for the tests
 * about what reaches the screen rather than what reaches the server. */
export function noComments(): CommentActions {
  return {
    on: () => [],
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

export function comment(over: Partial<CommentView> = {}): CommentView {
  return {
    id: "18cb-3731",
    path: "src/a.rs",
    side: "new",
    from: 1,
    to: 1,
    body: "Why this order?",
    pending: null,
    replies: [],
    waiting: null,
    ...over,
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
    look: null,
    lenses: [],
    folded: false,
    ...over,
  };
}
