// The map's hover popovers: the data-bs-* attributes the C# map components (MapImage, MapText,
// Planet, MapPopoverArea, MapForce) put on their elements, shown by Bootstrap through the shared
// usePopover hook (src/popover.ts), with MapComponent.PopoverTemplate as the frame.
import { usePopover } from "../../popover";

// MapComponent.PopoverStyle's default: the ink frame. Popovers that bring their own frame pass "".
export const DEFAULT_POPOVER_STYLE =
  "background-color:var(--ink-0);text-align:center;border:1px solid rgba(255,255,255,.16);background-clip:border-box;border-radius:var(--r-2);padding:3px 3px 3px 3px;box-shadow:var(--shadow-3);";

// Popover content: HTML, or a function that builds it (built when the element renders).
export type Html = string | (() => string | null) | null | undefined;

export function html(content: Html): string | null {
  return typeof content === "function" ? content() : content ?? null;
}

// MapComponent.PopoverTemplate.
function template(style: string): string {
  return `<div class="popover" role="tooltip" style="${style}"><div class="arrow"></div><h3 class="popover-header p-0"></h3><div class="popover-body p-0"></div></div>`;
}

export interface MapPopoverOptions {
  style?: string;
  delay?: number;
  placement?: string;
}

// The attributes and the element ref for a map popover; no attributes without content.
export function useMapPopover<T extends Element>(content: string | null, options: MapPopoverOptions = {}) {
  const ref = usePopover<T>(content);
  const attributes: Record<string, string> = content === null ? {} : {
    "data-bs-trigger": "hover",
    "data-bs-toggle": "popover",
    "data-bs-html": "true",
    "data-bs-animation": "false",
    "data-bs-delay": String(options.delay ?? 100),
    ...(options.placement ? { "data-bs-placement": options.placement } : {}),
    "data-container": "body",
    "data-bs-content": content,
    "data-bs-template": template(options.style ?? DEFAULT_POPOVER_STYLE),
  };
  return { ref, attributes };
}

// SetMapPopoversEnabled: intel popovers are off while a panel is pinned on the map.
export function setMapPopoversEnabled(enabled: boolean): void {
  const inner = document.getElementById("planetmap_inner");
  const bs = window.bootstrap as unknown as { Popover: { getInstance(e: Element): { enable(): void; disable(): void; hide(): void } | null } } | undefined;
  if (!inner || !bs) return;
  inner.querySelectorAll('[data-bs-toggle="popover"]').forEach((el) => {
    const p = bs.Popover.getInstance(el);
    if (!p) return;
    if (enabled) p.enable();
    else { p.hide(); p.disable(); }
  });
}
