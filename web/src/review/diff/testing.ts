import { vi } from "vitest";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView, FileView } from "@/api";

/** Comment actions that record the call and do nothing else, for the tests
 * about what reaches the screen rather than what reaches the server. */
export function noComments(): CommentActions {
  return {
    comments: [],
    on: () => [],
    unreadable: [],
    add: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
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
    published: null,
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
    notes: [],
    lineNotes: [],
    tags: [],
    skim: false,
    skimReason: null,
    ...over,
  };
}
