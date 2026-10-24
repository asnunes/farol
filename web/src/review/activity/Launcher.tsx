import { useEffect, useRef, useState } from "react";
import { Send, Sparkles } from "lucide-react";
import { cn } from "@/lib/utils";
import { Spinner } from "./Spinner";
import type { Activity } from "@/hooks/useActivity";

/** The box closed: a pill saying what is new and what the agents owe. */
export function Launcher({ ref, activity, open, onToggle }: LauncherProps) {
  const { working, fresh } = activity;
  const started = working.filter((w) => w.waiting.stage === "working").length;
  const asked = working.length - started;
  const bump = useBump(fresh);
  // All of them pending comments asked for, it is the ✦; any answer among
  // them, the plane.
  const AskedIcon = working.every((w) => w.waiting.stage === "working" || w.waiting.purpose === "pending")
    ? Sparkles
    : Send;

  return (
    <button
      ref={ref}
      className={cn(
        "launcher flex items-center gap-2 rounded-full border border-rule-strong bg-surface px-3 py-1.5 font-sans text-[0.78rem] text-ink-soft shadow-[0_6px_20px_rgb(0_0_0/0.4)] hover:text-ink",
        bump && "animate-[activity-bump_0.6s_ease]",
      )}
      aria-expanded={open}
      title="Agent activity — ."
      onClick={onToggle}
    >
      <span className={cn("size-[7px] rounded-full", fresh > 0 ? "bg-highlight" : "bg-faint")} aria-hidden="true" />
      {fresh === 0 && working.length === 0 && <span>No agent activity</span>}
      {fresh > 0 && <b className="font-semibold text-ink">{fresh} new</b>}
      {fresh > 0 && working.length > 0 && <span aria-hidden="true">·</span>}
      {asked > 0 && (
        <span className="inline-flex items-center gap-1.5 text-comment-ink">
          <AskedIcon className="size-3" aria-hidden="true" />
          {asked} waiting
        </span>
      )}
      {asked > 0 && started > 0 && <span aria-hidden="true">·</span>}
      {started > 0 && (
        <span className="inline-flex items-center gap-1.5 text-comment-ink">
          <Spinner />
          {started} working
        </span>
      )}
    </button>
  );
}

/** Whether the pill is hopping: something new arrived since the last count.
 * That hop is the whole of the interruption — the reader decides when to look. */
function useBump(fresh: number): boolean {
  const [bump, setBump] = useState(false);
  const before = useRef(fresh);

  useEffect(() => {
    const grew = fresh > before.current;
    before.current = fresh;
    if (!grew) return;
    setBump(true);
    const settle = window.setTimeout(() => setBump(false), 600);
    return () => window.clearTimeout(settle);
  }, [fresh]);

  return bump;
}

type LauncherProps = {
  ref: React.Ref<HTMLButtonElement>;
  activity: Activity;
  open: boolean;
  onToggle: () => void;
};
