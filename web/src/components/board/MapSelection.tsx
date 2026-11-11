// The map-driven selection of the action that owns the map (mapInteraction.ts):
// MapComponents/MapSelectionHighlights.razor (sources, the reachable region, the selection),
// MapSelectionOverlay.razor (arrows and the pinned panel) and StagedForces.razor (forces placed
// but not yet confirmed).
import type React from "react";
import { Fragment, useEffect, useRef } from "react";
import { num, px, sectorClipId, useBoard, wedgePath, type Board, type Point } from "./geometry";
import { useMapInteraction, usePin, type MapInteraction } from "./mapInteraction";
import { clampMapPin, listenForMapCancel, updateMapCursorArrows } from "./mapScripts";
import { setMapPopoversEnabled } from "./popover";
import { hasStarredSpecialForce } from "./primitives";

function involved(map: MapInteraction): number[] {
  return [...map.selectable, ...map.selected, ...map.targets, ...(map.target != null ? [map.target] : [])];
}

// The sector's area with a plain fill, for the masks and the wash.
function Shape({ b, l, fill, cls }: { b: Board; l: number; fill: string; cls?: string }) {
  return (
    <g clipPath={`url(#${sectorClipId(b.sector(l))})`}>
      <path d={b.border(b.territoryOf(l))} fill={fill} className={cls} />
    </g>
  );
}

function Highlight({ b, l, cls }: { b: Board; l: number; cls: string }) {
  if (b.hasBorder(l)) {
    const border = b.border(b.territoryOf(l));
    const sector = b.sector(l);
    const clip = sectorClipId(sector);
    return (
      <g className={cls}>
        <g clipPath={`url(#${clip})`}>
          <path d={border} className="fill" />
          <path d={border} className="core" />
          <path d={border} className="edge" />
        </g>
        {sector >= 0 && (
          <g clipPath={`url(#mapsel-t-${b.territoryOf(l)})`}>
            <g clipPath={`url(#${clip})`}>
              <path d={wedgePath(b, sector)} className="core radial" />
              <path d={wedgePath(b, sector)} className="edge radial" />
            </g>
          </g>
        )}
      </g>
    );
  }
  const center = b.getCenter(l);
  const radius = b.isHms(l) ? b.hmsRadius : Math.fround(2.2 * b.tokenRadius);
  return (
    <g className={cls}>
      <circle cx={num(center.X)} cy={num(center.Y)} r={num(radius)} className="fill" />
      <circle cx={num(center.X)} cy={num(center.Y)} r={num(radius)} className="edge" />
    </g>
  );
}

export function MapSelectionHighlights() {
  const b = useBoard();
  const map = useMapInteraction();
  if (map.owner == null || (map.selectable.size === 0 && map.selected.length === 0 && map.targets.size === 0)) return null;
  const { X: w, Y: h } = b.point("MapDimensions");
  const faction = b.factionColor(map.faction);
  // The pinned destination stays in the contour, so pinning it doesn't cut it out of the reachable region.
  const destinations = [...map.targets].filter((l) => !map.selected.includes(l) && b.hasCenter(l));
  const traced = destinations.filter((l) => b.hasBorder(l));
  const territories = [...new Set(involved(map).map((l) => b.territoryOf(l)))];
  const sectors = [...new Set(traced.map((l) => b.sector(l)).filter((s) => s >= 0))];
  const kinds = ["line", "halo"];
  return (
    <svg className="mapsel" width={w} height={h} viewBox={`0 0 ${w} ${h}`}
      style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", ["--c" as string]: faction }} pointerEvents="none">
      <defs>
        {territories.map((t) => <clipPath key={`t${t}`} id={`mapsel-t-${t}`}><path d={b.border(t)} /></clipPath>)}
        {kinds.map((kind) => (
          <Fragment key={kind}>
            {sectors.map((s) => (
              <mask key={s} id={`mapsel-grow-${kind}-${s}`} maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
                <path d={wedgePath(b, s)} className={`near-${kind}`} />
              </mask>
            ))}
            <mask id={`mapsel-near-${kind}`} maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
              {traced.map((l) => (
                <Fragment key={l}>
                  <g mask={`url(#mapsel-grow-${kind}-${b.sector(l)})`}>
                    <path d={b.border(b.territoryOf(l))} className={`near-${kind}`} />
                  </g>
                  {b.sector(l) >= 0 && (
                    <g clipPath={`url(#mapsel-t-${b.territoryOf(l)})`}>
                      <path d={wedgePath(b, b.sector(l))} className={`near-${kind}`} />
                    </g>
                  )}
                </Fragment>
              ))}
            </mask>
          </Fragment>
        ))}
        <mask id="mapsel-outside" maskUnits="userSpaceOnUse" x="0" y="0" width={w} height={h}>
          <rect x="0" y="0" width={w} height={h} fill="#fff" />
          {traced.map((l) => <Shape key={l} b={b} l={l} fill="#000" />)}
        </mask>
      </defs>
      {[...map.selectable].filter((l) => !map.selected.includes(l) && b.hasCenter(l)).map((l) => <Highlight key={`s${l}`} b={b} l={l} cls="mapsel-selectable" />)}
      <g className="mapsel-contour">
        {traced.map((l) => <Shape key={l} b={b} l={l} fill="#fff" cls="wash" />)}
        <g mask="url(#mapsel-outside)">
          <rect x="0" y="0" width={w} height={h} className="halo" mask="url(#mapsel-near-halo)" />
          <rect x="0" y="0" width={w} height={h} className="line" mask="url(#mapsel-near-line)" />
        </g>
      </g>
      {destinations.filter((l) => l !== map.target && !b.hasBorder(l)).map((l) => <Highlight key={`d${l}`} b={b} l={l} cls="mapsel-target" />)}
      {map.selected.filter((l) => b.hasCenter(l)).map((l) => <Highlight key={`x${l}`} b={b} l={l} cls="mapsel-selected" />)}
      {map.target != null && b.hasCenter(map.target) && <Highlight b={b} l={map.target} cls="mapsel-pinned" />}
    </svg>
  );
}

function trim(a: Point, c: Point, fromStart: number, fromEnd: number): [Point, Point] {
  const dx = c.X - a.X;
  const dy = c.Y - a.Y;
  const length = Math.sqrt(dx * dx + dy * dy);
  if (length < fromStart + fromEnd + 1) return [a, c];
  const ux = dx / length;
  const uy = dy / length;
  return [{ X: a.X + ux * fromStart, Y: a.Y + uy * fromStart }, { X: c.X - ux * fromEnd, Y: c.Y - uy * fromEnd }];
}

// MapSelectionOverlay.PinAnchor: the panel hangs above the destination, or below it near the top
// of the map, clear of the selected source tokens in its way.
function pinAnchor(b: Board, map: MapInteraction, starts: Point[], end: Point | null): [Point, boolean] {
  const radius = b.forceRadius;
  const clearance = 1.5 * radius;
  if (end == null) {
    const x = starts.reduce((s, p) => s + p.X, 0) / starts.length;
    const top = Math.min(...starts.map((p) => p.Y)) - clearance;
    if (top > 110) return [{ X: x, Y: top }, false];
    return [{ X: x, Y: Math.max(...starts.map((p) => p.Y)) + clearance }, true];
  }
  const halfWidth = 110;
  const height = 110;
  const sources = map.selected.length > 0 ? starts : [];
  const inColumn = sources.filter((p) => Math.abs(p.X - end.X) < halfWidth + radius);
  let above = end.Y - clearance;
  for (const p of inColumn.filter((q) => q.Y < end.Y).sort((q, r) => r.Y - q.Y)) {
    if (p.Y - radius < above && p.Y + radius > above - height) above = p.Y - clearance;
  }
  if (above > 110) return [{ X: end.X, Y: above }, false];
  let below = end.Y + clearance;
  for (const p of inColumn.filter((q) => q.Y >= end.Y).sort((q, r) => q.Y - r.Y)) {
    if (p.Y + radius > below && p.Y - radius < below + height) below = p.Y + clearance;
  }
  return [{ X: end.X, Y: below }, true];
}

function Pin() {
  return <>{usePin()}</>;
}

export function MapSelectionOverlay() {
  const b = useBoard();
  const map = useMapInteraction();
  const state = useRef({ suppressed: false, cancelable: false });
  const { X: w, Y: h } = b.point("MapDimensions");
  const faction = b.factionColor(map.faction);
  const starts = map.selected.filter((l) => b.hasCenter(l)).map((l) => b.getCenter(l));
  if (starts.length === 0 && map.origin != null) starts.push(map.origin);
  const end = map.target != null && b.hasCenter(map.target) ? b.getCenter(map.target) : map.endPoint;
  const showPin = map.pin != null && (end != null || map.selected.length > 0);

  useEffect(() => {
    const pinned = map.owner != null && map.target != null;
    if (pinned !== state.current.suppressed) {
      state.current.suppressed = pinned;
      setMapPopoversEnabled(!pinned);
    }
    if (map.owner != null && map.pin != null) clampMapPin();
    const cancelable = map.canCancel;
    if (cancelable !== state.current.cancelable) {
      state.current.cancelable = cancelable;
      listenForMapCancel(cancelable ? () => map.cancel() : null);
    }
    if (map.owner != null && map.selected.length > 0 && map.target == null && map.endPoint == null && map.showCursorArrow) updateMapCursorArrows();
  });
  useEffect(() => () => {
    if (state.current.suppressed) setMapPopoversEnabled(true);
    if (state.current.cancelable) listenForMapCancel(null);
  }, []);

  const arrows = map.owner != null && starts.length > 0 && (end != null || map.selected.length > 0);
  const cursorTargets = [...map.targets].filter((l) => !map.selected.includes(l)).join(",");
  let pin = null;
  if (map.owner != null && showPin) {
    const [anchor, below] = pinAnchor(b, map, starts, end);
    pin = (
      <div className={`map-pin ${below ? "below" : ""}`} style={{ left: px(anchor.X), top: px(anchor.Y), ["--c" as string]: faction }}>
        <Pin />
      </div>
    );
  }
  return (
    <>
      {arrows && (
        <svg className="maparrows" width={w} height={h} viewBox={`0 0 ${w} ${h}`}
          style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", ["--c" as string]: faction }} pointerEvents="none" data-targets={cursorTargets}>
          <defs>
            <marker id="mapsel-head" viewBox="0 0 12 12" refX="9" refY="6" markerWidth="4" markerHeight="4" orient="auto-start-reverse" markerUnits="strokeWidth" overflow="visible">
              <path d="M0,0 L12,6 L0,12 Z" className="head" />
            </marker>
          </defs>
          {starts.map((a, i) => {
            if (end != null) {
              const [p, q] = trim(a, end, 10, 12);
              return (
                <Fragment key={i}>
                  <line className="arrow-outline" x1={num(p.X)} y1={num(p.Y)} x2={num(q.X)} y2={num(q.Y)} />
                  <line className="arrow" x1={num(p.X)} y1={num(p.Y)} x2={num(q.X)} y2={num(q.Y)} markerEnd="url(#mapsel-head)" />
                </Fragment>
              );
            }
            if (!map.showCursorArrow) return null;
            return (
              <Fragment key={i}>
                <line className="arrow-outline mapsel-cursor" visibility="hidden" data-x1={num(a.X)} data-y1={num(a.Y)} x1={num(a.X)} y1={num(a.Y)} x2={num(a.X)} y2={num(a.Y)} />
                <line className="arrow mapsel-cursor" visibility="hidden" data-x1={num(a.X)} data-y1={num(a.Y)} x1={num(a.X)} y1={num(a.Y)} x2={num(a.X)} y2={num(a.Y)} markerEnd="url(#mapsel-head)" />
              </Fragment>
            );
          })}
        </svg>
      )}
      {pin}
    </>
  );
}

// StagedForces.razor: drawn to the right of the real tokens in the sector.
export function StagedForces() {
  const b = useBoard();
  const map = useMapInteraction();
  if (map.owner == null) return null;
  const diameter = 2 * b.forceRadius;
  const font = b.s("FORCETOKEN_FONT");
  const out: React.ReactNode[] = [];
  for (const [location, staged] of map.staged) {
    if (staged.forces + staged.specialForces <= 0 || !b.hasCenter(location)) continue;
    const center = b.getCenter(location);
    const here = b.view.board.forces.filter((f) => f.location === location);
    const columns = here.length === 0 ? 1 : Math.max(...here.map((f) => (f.forces > 0 ? 1 : 0) + (f.specialForces > 0 ? 1 : 0)));
    let x = center.X - 0.5 * diameter + Math.max(1, columns) * diameter + 0.2 * diameter;
    const y = center.Y - 0.5 * diameter;
    const token = (key: string, src: string, amount: number, tx: number, starred: boolean) => (
      <div key={key} className={`staged-force ${starred ? "force-token starred" : ""}`}
        style={{ position: "absolute", left: px(tx), top: px(y), width: px(diameter), height: px(diameter), ["--d" as string]: px(diameter) }}>
        <img src={src} alt="Staged force" />
        <span className="count" style={{ font }}>{amount}</span>
      </div>
    );
    if (staged.forces > 0) {
      out.push(token(`${location}n`, b.forceImage(staged.faction, false), staged.forces, x, false));
      x += 1.2 * diameter;
    }
    if (staged.specialForces > 0) {
      out.push(token(`${location}s`, b.forceImage(staged.faction, true), staged.specialForces, x, hasStarredSpecialForce(staged.faction)));
    }
  }
  return <>{out}</>;
}
