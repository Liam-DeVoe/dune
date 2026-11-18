// The map's hover popovers, as the map components (MapImage, MapText, Planet, MapPopoverArea,
// MapForce) set them up: the shared usePopover hook (src/popover.ts) in the map's frame (.map-pop).
import { setPopoversEnabled, usePopover, type Placement } from "../../popover";

// The default popover style: the ink frame. Popovers that bring their own frame pass "".
export const DEFAULT_POPOVER_STYLE =
  "background-color:var(--ink-0);text-align:center;border:1px solid rgba(255,255,255,.16);background-clip:border-box;border-radius:var(--r-2);padding:3px 3px 3px 3px;box-shadow:var(--shadow-3);";

// Popover content: HTML, or a function that builds it (built when the element renders).
export type Html = string | (() => string | null) | null | undefined;

export function html(content: Html): string | null {
  return typeof content === "function" ? content() : content ?? null;
}

export interface MapPopoverOptions {
  style?: string;
  delay?: number;
  placement?: Placement;
}

// The ref for an element with a map popover; none without content.
export function useMapPopover<T extends Element>(content: string | null, options: MapPopoverOptions = {}) {
  return usePopover<T>(content, {
    placement: options.placement ?? "right",
    delay: options.delay ?? 100,
    className: "map-pop",
    bodyClassName: "map-pop__body",
    style: options.style ?? DEFAULT_POPOVER_STYLE,
  });
}

// SetMapPopoversEnabled: intel popovers are off while a panel is pinned on the map.
export function setMapPopoversEnabled(enabled: boolean): void {
  const inner = document.getElementById("planetmap_inner");
  if (inner) setPopoversEnabled(inner, enabled);
}
