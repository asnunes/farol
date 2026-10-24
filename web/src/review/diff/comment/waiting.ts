import type { WaitingView } from "@/api";

/** What the agent owes, in the words the thread's badge and the activity box
 * both use — written once, so the two never describe the same wait apart.
 *
 * Said from the agent's side, because that is what the reader wants to know:
 * "Asked Codex" rather than "waiting for an answer". The purpose is spelt out
 * once, when it is asked for; after that the ✦ the Generate button carries is
 * what says a pending comment is on its way. */
export function waitingLine(waiting: WaitingView): string {
  const started = waiting.stage === "working";
  if (waiting.purpose === "pending") {
    return started ? `${waiting.on} is drafting it` : `Asked ${waiting.on} for a pending comment`;
  }
  return started ? `${waiting.on} is answering` : `Asked ${waiting.on}`;
}
