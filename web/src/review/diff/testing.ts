import { vi } from "vitest";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** Comment actions that record the call and do nothing else, for the tests
 * about what reaches the screen rather than what reaches the server. */
export function noComments(): CommentActions {
  return {
    comments: [],
    add: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

export function comment(over: Partial<CommentView> = {}): CommentView {
  return {
    id: "18cb-3731",
    path: "src/a.rs",
    from: 1,
    to: 1,
    body: "Why this order?",
    ...over,
  };
}
