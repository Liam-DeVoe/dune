// Hover popovers: the HTML of popups.ts in a box beside an element while the mouse is over it,
// as the C# client showed them with Bootstrap (EnablePopover, RefreshPopover, RemovePopover,
// PopoverOptions in its index.html). The box is a native popover, so it is in the top layer and
// shows over an open modal too. Placement follows the Popper setup Bootstrap used: on the preferred
// side, GAP away from the element; on another side when it doesn't fit there; then shifted along
// both axes to stay MARGIN inside the window. Styles in components/_popover.scss.
import { useEffect, useRef, type RefObject } from "react";

export type Placement = "top" | "bottom" | "left" | "right";

export interface PopoverOptions {
  // The preferred side, or "auto" for the side with the most room. Default "right".
  placement?: Placement | "auto";
  // Milliseconds from entering the element to showing, and from leaving it to hiding. Default 0.
  delay?: number;
  // Fade in and out.
  animated?: boolean;
  // Extra classes and inline style of the box and of its body (the map's frame, board/popover.ts).
  className?: string;
  bodyClassName?: string;
  style?: string;
}

// RichText.razor's popovers on text, and Image.razor's on images.
export const TEXT_POPOVER: PopoverOptions = { delay: 100 };
export const IMAGE_POPOVER: PopoverOptions = { placement: "auto", animated: true };

const GAP = 8;     // px between the element and the box
const MARGIN = 8;  // px kept clear of the window's edges
const FADE = 150;  // ms, the fade of an animated box (.popover--fade)

const SIDES: Placement[] = ["top", "bottom", "right", "left"];
// Where Popper's flip looks after the preferred side.
const FALLBACKS: Placement[] = ["top", "right", "bottom", "left"];
const opposite = (p: Placement): Placement =>
  p === "top" ? "bottom" : p === "bottom" ? "top" : p === "left" ? "right" : "left";

interface Box { x: number; y: number; w: number; h: number }

// The box's position on side `p` of the element `r`, centred along that side.
function at(p: Placement, r: DOMRect, w: number, h: number): Box {
  switch (p) {
    case "top": return { x: r.left + (r.width - w) / 2, y: r.top - h - GAP, w, h };
    case "bottom": return { x: r.left + (r.width - w) / 2, y: r.bottom + GAP, w, h };
    case "left": return { x: r.left - w - GAP, y: r.top + (r.height - h) / 2, w, h };
    case "right": return { x: r.right + GAP, y: r.top + (r.height - h) / 2, w, h };
  }
}

// How far the box sticks out of the window on each side (negative: room left).
function overflow(b: Box, vw: number, vh: number): Record<Placement, number> {
  return { top: -b.y, left: -b.x, bottom: b.y + b.h - vh, right: b.x + b.w - vw };
}

// Popper's flip: the first side where the box fits whole; else the first where it fits on its side
// and towards its start; else the first where it fits on its side; else the preferred one.
function chooseSide(placement: Placement | "auto", r: DOMRect, w: number, h: number, vw: number, vh: number): Placement {
  const preferred = placement === "auto"
    ? [...SIDES].sort((a, b) => overflow(at(a, r, w, h), vw, vh)[a] - overflow(at(b, r, w, h), vw, vh)[b])
    : [placement];
  const order = [...preferred, ...FALLBACKS];
  const checks = order.map((p) => {
    const o = overflow(at(p, r, w, h), vw, vh);
    const vertical = p === "top" || p === "bottom";
    let start: Placement = vertical ? "left" : "top";
    if (vertical ? r.width > w : r.height > h) start = opposite(start);
    return [o[p] <= 0, o[start] <= 0, o[opposite(start)] <= 0];
  });
  for (const n of [3, 2, 1]) {
    const i = checks.findIndex((c) => c.slice(0, n).every((ok) => ok));
    if (i >= 0) return order[i];
  }
  return order[0];
}

const clamp = (v: number, min: number, max: number) => Math.max(min, Math.min(v, max));
const round = (v: number) => Math.round(v * devicePixelRatio) / devicePixelRatio;

let uid = 0;
const popovers = new Set<HoverPopover>();

class HoverPopover {
  private box: HTMLDivElement | null = null;
  private body: HTMLDivElement | null = null;
  private resize: ResizeObserver | null = null;
  private timer = 0;
  private hovered = false;
  enabled = true;

  constructor(readonly element: Element, private content: string, private readonly options: PopoverOptions) {
    element.addEventListener("mouseenter", this.enter);
    element.addEventListener("mouseleave", this.leave);
    popovers.add(this);
  }

  dispose() {
    clearTimeout(this.timer);
    this.element.removeEventListener("mouseenter", this.enter);
    this.element.removeEventListener("mouseleave", this.leave);
    this.hide(false);
    popovers.delete(this);
  }

  // RefreshPopover: an open box takes the new content in place, so a redraw doesn't close it.
  setContent(content: string) {
    this.content = content;
    if (this.body) {
      this.body.innerHTML = content;
      showLongForms(this.body);
      this.place();
    }
  }

  private later(f: () => void) {
    clearTimeout(this.timer);
    this.timer = window.setTimeout(f, this.options.delay ?? 0);
  }

  private enter = () => {
    const shown = this.box !== null;
    this.hovered = true;
    if (!shown) this.later(() => { if (this.hovered) this.show(); });
  };

  private leave = () => {
    this.hovered = false;
    this.later(() => { if (!this.hovered) this.hide(); });
  };

  private show() {
    if (this.box || !this.enabled || this.content === "" || !this.element.isConnected) return;
    const box = document.createElement("div");
    box.id = `popover${++uid}`;
    box.className = ["popover", this.options.className, this.options.animated ? "popover--fade" : ""].filter((c) => c).join(" ");
    box.setAttribute("role", "tooltip");
    if (this.options.style) box.setAttribute("style", this.options.style);
    box.setAttribute("popover", "manual");
    const body = document.createElement("div");
    body.className = ["popover__body", this.options.bodyClassName].filter((c) => c).join(" ");
    body.innerHTML = this.content;
    showLongForms(body);
    box.appendChild(body);
    document.body.appendChild(box);
    if (typeof box.showPopover === "function") box.showPopover();
    this.box = box;
    this.body = body;
    this.place();
    this.element.setAttribute("aria-describedby", box.id);
    if (this.options.animated) box.classList.add("popover--shown");
    // Card art often loads after the box opens.
    this.resize = new ResizeObserver(() => this.place());
    this.resize.observe(box);
  }

  hide(animate = this.options.animated) {
    const box = this.box;
    if (!box) return;
    this.box = null;
    this.body = null;
    this.resize?.disconnect();
    this.resize = null;
    this.element.removeAttribute("aria-describedby");
    if (animate) {
      box.classList.remove("popover--shown");
      window.setTimeout(() => box.remove(), FADE);
    } else {
      box.remove();
    }
  }

  private place() {
    const box = this.box;
    if (!box) return;
    const r = this.element.getBoundingClientRect();
    const w = box.offsetWidth;
    const h = box.offsetHeight;
    const vw = document.documentElement.clientWidth;
    const vh = document.documentElement.clientHeight;
    const b = at(chooseSide(this.options.placement ?? "right", r, w, h, vw, vh), r, w, h);
    const x = clamp(b.x, MARGIN, vw - MARGIN - w);
    const y = clamp(b.y, MARGIN, vh - MARGIN - h);
    box.style.transform = `translate(${round(x)}px, ${round(y)}px)`;
  }
}

// While Shift is held, popovers with a longer form show it: the faction popover takes
// pop-faction--full, its full rules sheet (_pop.scss). Applied to the popovers open when Shift
// goes down or up, and to each popover as it opens. Cleared on blur, since the keyup then never
// arrives.
let listening = false;
let shiftHeld = false;
function showLongForms(root: ParentNode) {
  for (const el of root.querySelectorAll(".pop-faction")) el.classList.toggle("pop-faction--full", shiftHeld);
}
function setShift(held: boolean) {
  if (held === shiftHeld) return;
  shiftHeld = held;
  showLongForms(document);
}
function listen() {
  if (listening || typeof document === "undefined") return;
  listening = true;
  document.addEventListener("keydown", (e) => { if (e.key === "Shift") setShift(true); });
  document.addEventListener("keyup", (e) => { if (e.key === "Shift") setShift(false); });
  window.addEventListener("blur", () => setShift(false));
}

// A hover popover on the element while it is mounted and `content` isn't null. When the content
// changes, an open popover is updated in place. The options are read once.
export function usePopover<T extends Element>(content: string | null, options: PopoverOptions = {}): RefObject<T | null> {
  const ref = useRef<T>(null);
  const instance = useRef<HoverPopover | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el || content === null) return;
    listen();
    const popover = new HoverPopover(el, content, options);
    instance.current = popover;
    return () => {
      popover.dispose();
      instance.current = null;
    };
    // Created once per element; content changes go through the effect below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content === null]);
  useEffect(() => {
    if (content !== null) instance.current?.setContent(content);
  }, [content]);
  return ref;
}

// Hides the popovers of the elements inside `root`, as a modal closes.
export function hidePopoversIn(root: Element): void {
  popovers.forEach((p) => { if (root.contains(p.element)) p.hide(); });
}

// Turns the popovers of the elements inside `root` on or off; turning one off hides it.
export function setPopoversEnabled(root: Element, enabled: boolean): void {
  popovers.forEach((p) => {
    if (!root.contains(p.element)) return;
    p.enabled = enabled;
    if (!enabled) p.hide();
  });
}
