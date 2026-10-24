import { LoaderCircle, Send, Sparkles } from "lucide-react";
import { cn } from "@/lib/utils";
import type { WaitingView } from "@/api";

/** The mark beside a wait: the send plane for an answer asked, the ✦ of the
 * Generate button for a pending comment asked, and either turning once the
 * agent has started on it. */
export function WaitingIcon({ waiting, className }: WaitingIconProps) {
  const started = waiting.stage === "working";
  const size = cn("size-3.5", className);
  if (waiting.purpose === "pending") {
    return <Sparkles className={cn(size, started && "animate-spin")} aria-hidden="true" />;
  }
  return started ? (
    <LoaderCircle className={cn(size, "animate-spin")} aria-hidden="true" />
  ) : (
    <Send className={size} aria-hidden="true" />
  );
}

type WaitingIconProps = { waiting: WaitingView; className?: string };
