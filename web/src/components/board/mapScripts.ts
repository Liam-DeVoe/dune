// The board's scripts from the C# client's wwwroot/index.html, as a module: SetPlanetMapScale
// (fit the board to its area), StartMapHover/ShowMapHover (the sector under the mouse),
// UpdateMapCursorArrows and TrimMapArrow (the arrow to the hovered destination),
// ListenForMapCancel (Escape or a right click cancels a half-made selection), ShowMapRefs (a
// place chip in a sentence lights its sectors) and ClampMapPin (the pinned panel stays on the page).

// SetPlanetMapScale: on desktop the board is scaled to fit its area's height, cropped to the ring
// and the seat tokens around it; the corner pieces move out to the area's edges through
// --map-dx-l and friends. Below 992px the board is as wide as its column and the page scrolls.
export function setPlanetMapScale(): void {
  try {
    const planetmap = document.getElementById("planetmap");
    const inner = document.getElementById("planetmap_inner");
    if (planetmap == null || inner == null) return;
    const area = planetmap.parentElement!;
    const filler = document.getElementById("planetmap-filler")!;
    const mapW = inner.offsetWidth;
    const mapH = inner.offsetHeight;
    const W = area.clientWidth;
    const H = area.clientHeight;
    const fit = window.innerWidth >= 992 && H > 0;
    let s: number;
    let tx = 0, ty = 0, l = 0, r = 0, t = 0, b = 0, bar = 0;
    if (fit) {
      const top = Number(inner.dataset.viewTop) || 0;
      const bottom = Number(inner.dataset.viewBottom) || mapH;
      s = Math.min(H / (bottom - top), W / mapW);
      tx = (W - mapW * s) / 2;
      ty = (H - (bottom - top) * s) / 2 - top * s;
      l = -tx / s;
      r = (W - tx) / s - mapW;
      t = -ty / s;
      b = (H - ty) / s - mapH;
      const menu = area.querySelector<HTMLElement>(":scope > .menu-bar");
      bar = menu != null ? menu.offsetHeight / s : 0;
      filler.style.height = "0px";
    } else {
      s = W / mapW;
      filler.style.height = s * mapH + "px";
    }
    planetmap.style.transformOrigin = "0 0";
    planetmap.style.transform = "translate(" + tx + "px, " + ty + "px) scale(" + s + ")";
    inner.style.setProperty("--map-dx-l", l + "px");
    inner.style.setProperty("--map-dx-r", r + "px");
    inner.style.setProperty("--map-dy-t", t + "px");
    inner.style.setProperty("--map-dy-b", b + "px");
    inner.style.setProperty("--map-bar", bar + "px");
    clampMapPin();
  } catch {
    // As in C#: a failed measurement leaves the board as it was.
  }
}

let resizeListening = false;
export function listenForResize(): void {
  if (resizeListening) return;
  resizeListening = true;
  window.addEventListener("resize", setPlanetMapScale);
}

// TrimMapArrow (and MapSelectionOverlay.Trim): keeps an arrow off the tokens at either end.
export function trimMapArrow(a: { x: number; y: number }, b: { x: number; y: number }, fromStart: number, fromEnd: number) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = Math.hypot(dx, dy);
  if (length < fromStart + fromEnd + 1) return [a, b];
  const ux = dx / length;
  const uy = dy / length;
  return [{ x: a.x + ux * fromStart, y: a.y + uy * fromStart }, { x: b.x - ux * fromEnd, y: b.y - uy * fromEnd }];
}

let mapCursorTarget: string | null = null;

// UpdateMapCursorArrows: points the cursor arrows at the last valid destination hovered.
export function updateMapCursorArrows(): void {
  const inner = document.getElementById("planetmap_inner");
  if (inner == null) return;
  const arrows = inner.querySelector<SVGElement>(".maparrows");
  const lines = inner.querySelectorAll<SVGLineElement>("line.mapsel-cursor");
  if (arrows == null || lines.length === 0) {
    mapCursorTarget = null;
    return;
  }
  const targets = new Set((arrows.dataset.targets || "").split(",").filter((id) => id !== ""));
  const hovered = inner.querySelector<SVGElement>(".maphover .hover-shape.on");
  if (hovered != null && targets.has(hovered.dataset.locationId!)) mapCursorTarget = hovered.dataset.locationId!;
  const shape = mapCursorTarget != null && targets.has(mapCursorTarget)
    ? inner.querySelector<SVGElement>(`.maphover .hover-shape[data-location-id="${mapCursorTarget}"]`)
    : null;
  const end = shape != null && shape.dataset.cx ? { x: Number(shape.dataset.cx), y: Number(shape.dataset.cy) } : null;
  lines.forEach((line) => {
    if (end == null) {
      line.setAttribute("visibility", "hidden");
      return;
    }
    const start = { x: Number(line.dataset.x1), y: Number(line.dataset.y1) };
    const segment = trimMapArrow(start, end, 10, 12);
    line.setAttribute("x1", String(segment[0].x));
    line.setAttribute("y1", String(segment[0].y));
    line.setAttribute("x2", String(segment[1].x));
    line.setAttribute("y2", String(segment[1].y));
    line.removeAttribute("visibility");
  });
}

let mapHoverInner: HTMLElement | null = null;

function mapHoverLocation(element: EventTarget | null): string | null {
  const el = element as Element | null;
  if (el == null || el.closest == null) return null;
  const group = el.closest<SVGElement>("g[data-location-id]:not(.hover-shape)");
  return group == null ? null : group.dataset.locationId ?? null;
}

function showMapHover(locationId: string | null): void {
  if (mapHoverInner == null) return;
  mapHoverInner.querySelectorAll(".maphover .hover-shape.on").forEach((shape) => shape.classList.remove("on"));
  if (locationId == null) return;
  const shape = mapHoverInner.querySelector(`.maphover .hover-shape[data-location-id="${locationId}"]`);
  if (shape != null) shape.classList.add("on");
  updateMapCursorArrows();
}

// StartMapHover: shows the sector under the mouse. Called after every render of the board.
export function startMapHover(): void {
  const inner = document.getElementById("planetmap_inner");
  if (inner == null) return;
  if (mapHoverInner !== inner) {
    mapHoverInner = inner;
    inner.addEventListener("mouseover", (e) => {
      const id = mapHoverLocation(e.target);
      if (id != null) showMapHover(id);
    });
    inner.addEventListener("mouseout", (e) => {
      if (mapHoverLocation(e.target) != null) showMapHover(null);
    });
  }
  showMapHover(mapHoverLocation(inner.querySelector("g[data-location-id]:not(.hover-shape):hover")));
}

let cancelKey: ((e: KeyboardEvent) => void) | null = null;
let cancelMenu: ((e: MouseEvent) => void) | null = null;
let cancelMenuTarget: HTMLElement | null = null;

// ListenForMapCancel: only while there is something to cancel, so the browser's context menu
// keeps working the rest of the time. Null stops listening.
export function listenForMapCancel(cancel: (() => void) | null): void {
  if (cancelKey != null) {
    document.removeEventListener("keydown", cancelKey);
    cancelKey = null;
  }
  if (cancelMenu != null) {
    cancelMenuTarget!.removeEventListener("contextmenu", cancelMenu);
    cancelMenu = null;
    cancelMenuTarget = null;
  }
  if (cancel == null) return;
  cancelKey = (e) => {
    if (e.key !== "Escape") return;
    if (document.querySelector('.modal.show, [aria-expanded="true"]') != null) return;
    const t = e.target as Element | null;
    const outsideMap = t == null || t.closest == null || t.closest("#planetmap_inner") == null;
    if (outsideMap && t != null && t.matches != null && t.matches("input, textarea, select, [contenteditable]")) return;
    cancel();
  };
  document.addEventListener("keydown", cancelKey);
  const inner = document.getElementById("planetmap_inner");
  if (inner == null) return;
  cancelMenu = (e) => {
    e.preventDefault();
    cancel();
  };
  cancelMenuTarget = inner;
  inner.addEventListener("contextmenu", cancelMenu);
}

// ClampMapPin: shifts a pinned panel near the page's edge back inside, in map units.
export function clampMapPin(): void {
  const pin = document.querySelector<HTMLElement>(".map-pin");
  if (pin == null) return;
  const inner = document.getElementById("planetmap_inner");
  const scale = inner != null && inner.offsetWidth > 0 ? inner.getBoundingClientRect().width / inner.offsetWidth : 1;
  const current = parseFloat(pin.style.getPropertyValue("--dx")) || 0;
  const rect = pin.getBoundingClientRect();
  const left = rect.left - current * scale;
  const right = rect.right - current * scale;
  const margin = 8;
  const max = document.documentElement.clientWidth - margin;
  let dx = 0;
  if (left < margin) dx = margin - left;
  else if (right > max) dx = max - right;
  pin.style.setProperty("--dx", dx / scale + "px");
}

// ShowMapRefs: while the mouse is over a chip that names sectors (data-ref-locations) and maybe
// a faction (data-ref-faction), those sectors show their hover shape and anything tagged
// data-faction-of that faction gets .ref-hi. A log line with data-ref-arrow draws its journey.
function showMapRefs(chip: HTMLElement | null): void {
  document.querySelectorAll<SVGElement>(".maphover .hover-shape.ref").forEach((shape) => {
    shape.classList.remove("ref", "joined-start", "joined-end");
    shape.style.removeProperty("--c");
  });
  document.querySelectorAll(".ref-hi").forEach((el) => el.classList.remove("ref-hi"));
  const lines = document.querySelector(".maprefs .lines");
  if (lines != null) lines.replaceChildren();
  if (chip == null) return;
  const color = getComputedStyle(chip).getPropertyValue("--fc").trim();
  const journey = chip.hasAttribute("data-ref-arrow") ? chip : null;
  if (journey != null && lines != null) {
    const arrowColor = getComputedStyle(journey).getPropertyValue("--fc").trim();
    (lines.closest("svg") as SVGElement).style.setProperty("--c", arrowColor !== "" ? arrowColor : "#fff");
    journey.dataset.refArrow!.split(";").forEach((segment) => {
      const n = segment.split(",").map(Number);
      if (n.length !== 4) return;
      const ends = trimMapArrow({ x: n[0], y: n[1] }, { x: n[2], y: n[3] }, 10, 12);
      for (const cls of ["arrow-outline", "arrow"]) {
        const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
        line.setAttribute("class", cls);
        line.setAttribute("x1", String(ends[0].x));
        line.setAttribute("y1", String(ends[0].y));
        line.setAttribute("x2", String(ends[1].x));
        line.setAttribute("y2", String(ends[1].y));
        if (cls === "arrow") line.setAttribute("marker-end", "url(#mapref-head)");
        lines.appendChild(line);
      }
    });
  }
  const lit: SVGElement[] = [];
  (chip.dataset.refLocations ?? "").split(",").forEach((id) => {
    if (id === "") return;
    document.querySelectorAll<SVGElement>(`.maphover .hover-shape[data-location-id="${id}"]`).forEach((shape) => {
      shape.classList.add("ref");
      if (color !== "") shape.style.setProperty("--c", color);
      lit.push(shape);
    });
  });
  const sectors = Number(document.querySelector<SVGElement>(".maphover")?.dataset.sectors) || 18;
  const litKeys = new Set(lit.map((shape) => shape.dataset.territoryId + ":" + shape.dataset.sector));
  lit.forEach((shape) => {
    const sector = Number(shape.dataset.sector);
    if (!(sector >= 0)) return;
    const t = shape.dataset.territoryId;
    if (litKeys.has(t + ":" + ((sector + sectors - 1) % sectors))) shape.classList.add("joined-start");
    if (litKeys.has(t + ":" + ((sector + 1) % sectors))) shape.classList.add("joined-end");
  });
  const faction = chip.dataset.refFaction;
  if (faction) document.querySelectorAll(`[data-faction-of="${faction}"]`).forEach((el) => el.classList.add("ref-hi"));
}

let refsListening = false;
export function listenForMapRefs(): void {
  if (refsListening) return;
  refsListening = true;
  document.addEventListener("mouseover", (e) => {
    const t = e.target as Element;
    const chip = t.closest == null ? null : t.closest<HTMLElement>("[data-ref-locations]");
    if (chip != null) showMapRefs(chip);
  });
  document.addEventListener("mouseout", (e) => {
    const t = e.target as Element;
    const chip = t.closest == null ? null : t.closest<HTMLElement>("[data-ref-locations]");
    if (chip != null && !chip.contains(e.relatedTarget as Node | null)) showMapRefs(null);
  });
}
