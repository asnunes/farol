import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { usePageVisible } from "@/hooks/usePageVisible";
import type { ReactNode } from "react";

/** Evict code DOM outside the reading window without moving the scroll position. */
export function DiffWindow({
  children,
  rows,
  pinned = false,
  layout,
}: DiffWindowProps) {
  const box = useRef<HTMLDivElement>(null);
  const [near, setNear] = useState(typeof IntersectionObserver === "undefined");
  const [focused, setFocused] = useState(false);
  const [selected, setSelected] = useState(false);
  const height = useRef<number | null>(null);
  const canvas = useRef(0);
  const width = useRef(0);
  const [measurement, resized] = useState(0);
  const previousLayout = useRef(layout);
  const visible = usePageVisible();
  if (previousLayout.current !== layout) {
    previousLayout.current = layout;
    height.current = null;
    canvas.current = 0;
    width.current = 0;
  }
  const mounted = pinned || focused || selected || (near && visible);

  useEffect(() => {
    const el = box.current;
    if (!el || typeof IntersectionObserver === "undefined") return;
    return observe(el, setNear);
  }, []);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => {
      const available = el.parentElement?.clientWidth ?? 0;
      if (mounted) {
        const contentWidth = Math.max(
          available,
          el.firstElementChild?.scrollWidth ?? 0,
        );
        const changed =
          width.current !== available || canvas.current !== contentWidth;
        height.current = el.getBoundingClientRect().height;
        canvas.current = contentWidth;
        width.current = available;
        if (changed) resized((n) => n + 1);
      } else {
        if (width.current && width.current !== available) {
          height.current = null;
          canvas.current = 0;
          resized((n) => n + 1);
        }
        width.current = available;
      }
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      if (
        entries.some(
          (entry) =>
            entry.target === el || entry.contentRect.width !== width.current,
        )
      )
        measure();
    });
    observer.observe(el);
    if (el.parentElement) observer.observe(el.parentElement);
    return () => observer.disconnect();
  }, [mounted, layout, measurement]);

  useEffect(() => {
    const keepSelection = () => {
      const selection = window.getSelection();
      const el = box.current;
      setSelected(
        !!(
          el &&
          selection &&
          !selection.isCollapsed &&
          selection.containsNode(el, true)
        ),
      );
    };
    document.addEventListener("selectionchange", keepSelection);
    return () => document.removeEventListener("selectionchange", keepSelection);
  }, []);

  const estimate = Math.max(1, rows) * 22;
  return (
    <div
      ref={box}
      className="diff-window"
      data-rendered={mounted ? "true" : "false"}
      onFocusCapture={() => setFocused(true)}
      onBlurCapture={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null))
          setFocused(false);
      }}
      style={{
        height: mounted ? undefined : (height.current ?? estimate),
        minWidth: canvas.current || undefined,
      }}
    >
      {/* The outer canvas remains observable when scrolled sideways. Prose
          inside still wraps at the pane width, not at the longest code line. */}
      {mounted ? (
        <div style={{ width: width.current || undefined }}>{children}</div>
      ) : null}
    </div>
  );
}

/** One observer per reading pane, rather than one observer per chunk. */
function observe(element: Element, changed: (near: boolean) => void) {
  const root = element.closest(".pane");
  let group = observers.get(root);
  if (!group) {
    const callbacks = new Map<Element, (near: boolean) => void>();
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries)
          callbacks.get(entry.target)?.(entry.isIntersecting);
      },
      { root, rootMargin: "800px 0px" },
    );
    group = { callbacks, observer };
    observers.set(root, group);
  }
  group.callbacks.set(element, changed);
  group.observer.observe(element);
  return () => {
    group.callbacks.delete(element);
    group.observer.unobserve(element);
    if (group.callbacks.size === 0) {
      group.observer.disconnect();
      observers.delete(root);
    }
  };
}

const observers = new Map<Element | null, ObservedPane>();

type DiffWindowProps = {
  children: ReactNode;
  rows: number;
  pinned?: boolean;
  layout: string;
};

type ObservedPane = {
  observer: IntersectionObserver;
  callbacks: Map<Element, (near: boolean) => void>;
};
