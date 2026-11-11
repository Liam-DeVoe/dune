// Bootstrap popovers, as the C# client's index.html drives them (EnablePopover, RefreshPopover,
// RemovePopover, PopoverOptions). Bootstrap itself is the C# client's bundle,
// /js/bootstrap.bundle.min.js, loaded by index.html, so popovers look and place as in C#.
import { useEffect, useRef, type RefObject } from "react";

interface BootstrapPopover {
  dispose(): void;
  update(): void;
  setContent(content: Record<string, string>): void;
  _isShown(): boolean;
  tip: HTMLElement | null;
}

interface Bootstrap {
  Popover: {
    new (element: Element, options: object): BootstrapPopover;
    getInstance(element: Element): BootstrapPopover | null;
  };
}

declare global {
  interface Window { bootstrap?: Bootstrap }
}

// PopoverOptions: popper may slide a popover along its element until it fits the window.
function popoverOptions() {
  return {
    sanitize: false,
    popperConfig: (defaults: { modifiers: { name: string; options?: object }[] }) => ({
      ...defaults,
      modifiers: defaults.modifiers.map((m) => m.name === "preventOverflow"
        ? { ...m, options: { ...m.options, tether: false, altAxis: true, padding: 8 } }
        : m),
    }),
  };
}

// Popper places a popover once, when it opens; card art often loads after that. Place it again
// whenever its size changes (index.html, 'inserted.bs.popover').
let listening = false;
function listen() {
  if (listening || typeof document === "undefined") return;
  listening = true;
  document.addEventListener("inserted.bs.popover", (e) => {
    const target = e.target as Element;
    const popover = window.bootstrap?.Popover.getInstance(target);
    const tip = popover?.tip;
    if (!popover || !tip) return;
    const observer = new ResizeObserver(() => popover.update());
    observer.observe(tip);
    target.addEventListener("hidden.bs.popover", () => observer.disconnect(), { once: true });
  });
  // While Shift is held, popovers with a longer form show it (the faction popover's full rules
  // sheet, .pop-faction in _pop.scss). Cleared on blur, since the keyup then never arrives.
  document.addEventListener("keydown", (e) => { if (e.key === "Shift") document.body.classList.add("shift-held"); });
  document.addEventListener("keyup", (e) => { if (e.key === "Shift") document.body.classList.remove("shift-held"); });
  window.addEventListener("blur", () => document.body.classList.remove("shift-held"));
}

// The attributes C# RichText.razor and Image.razor put on an element with a rich popover.
export function popoverAttributes(content: string, kind: "text" | "image"): Record<string, string> {
  return kind === "text"
    ? { "data-bs-trigger": "hover", "data-bs-toggle": "popover", "data-bs-html": "true", "data-bs-animation": "false", "data-bs-delay": "100", "data-container": "body", "data-bs-content": content }
    : { "data-bs-toggle": "popover", "data-bs-animation": "true", "data-bs-html": "true", "data-bs-placement": "auto", "data-bs-trigger": "hover", "data-bs-content": content };
}

// Enables a Bootstrap popover on the element while it is mounted. When the content changes, an
// open popover is updated in place (RefreshPopover), so a redraw doesn't close it.
export function usePopover<T extends Element>(content: string | null): RefObject<T | null> {
  const ref = useRef<T>(null);
  const instance = useRef<BootstrapPopover | null>(null);
  useEffect(() => {
    const el = ref.current;
    const bs = window.bootstrap;
    if (!el || !bs || content === null) return;
    listen();
    instance.current = new bs.Popover(el, popoverOptions());
    return () => {
      instance.current?.dispose();
      instance.current = null;
    };
    // Created once per element; content changes go through the effect below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content === null]);
  useEffect(() => {
    const p = instance.current;
    if (p && content !== null) p.setContent({ ".popover-body": content });
  }, [content]);
  return ref;
}
