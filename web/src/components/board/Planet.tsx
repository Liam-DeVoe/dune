// The planet's sectors: MapComponents/Planet.razor (sector clips, storm, highlighted territories
// and the clickable locations), MapHover.razor (the sector under the mouse) and RecentBlows.razor
// (where spice blew this turn).
import { Fragment } from "react";
import { useBoard, sectorClipId, wedgeEdges, wedgePath, SECTORS, num, type Board } from "./geometry";
import { useMapPopover } from "./popover";
import { locationIntel } from "./intel";
import { useMapClick } from "./mapInteraction";

function locationColor(b: Board, id: number): string {
  const board = b.view.board;
  const sector = b.sector(id);
  if (board.sectorInStorm >= 0 && sector === board.sectorInStorm) return b.s("STORM_OVERLAY_COLOR");
  if (board.atomicsAftermath === b.territoryOf(id)) return b.s("BLOWNSHIELDWALL_OVERLAY_COLOR");
  const moves = board.stormPrescienceMoves;
  if (moves != null) {
    for (let i = 1; i <= moves; i++) {
      if ((board.sectorInStorm + i) % SECTORS === sector) return b.s("STORM_PRESCIENCE_OVERLAY_COLOR");
    }
  }
  return "transparent";
}

export function Planet() {
  const b = useBoard();
  const click = useMapClick();
  const { X: w, Y: h } = b.point("MapDimensions");
  const scaleX = Math.fround(563 / w);
  const scaleY = Math.fround(626 / h);
  const { X: cx, Y: cy } = b.center;
  const r = b.planetRadius;
  const map = b.skin.statics.map;
  const board = b.view.board;
  const clips = [];
  for (let i = 0; i < SECTORS; i++) {
    const sx = Math.cos(2 * Math.PI * (6 - i) / SECTORS) * r + cx;
    const sy = Math.sin(2 * Math.PI * (6 - i) / SECTORS) * r + cy;
    const ex = Math.cos(2 * Math.PI * (5 - i) / SECTORS) * r + cx;
    const ey = Math.sin(2 * Math.PI * (5 - i) / SECTORS) * r + cy;
    const d = `M ${num(scaleX * cx)},${num(scaleY * cy)} L ${num(scaleX * sx)},${num(scaleY * sy)} A ${num(scaleX * r)} ${num(scaleY * r)} ${num(360 / SECTORS)} 0 0 ${num(scaleX * ex)},${num(scaleY * ey)} Z`;
    clips.push(<clipPath key={i} id={sectorClipId(i)}><path d={d} /></clipPath>);
  }
  const hmsTerritory = b.territoryOf(map.hiddenMobileStronghold);
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ position: "absolute", left: 0, top: 0, height: "100%", width: "100%" }} pointerEvents="none">
      {clips}
      <clipPath id={sectorClipId(-1)}><rect x="0" y="0" width={w} height={h} /></clipPath>
      {board.shieldWallDestroyed && (
        <>
          <path d={b.border(map.shieldWall)} fill={b.s("BLOWNSHIELDWALL_OVERLAY_COLOR")} fillOpacity="1" stroke="transparent" />
          <path d={b.border(b.territoryOf(map.arrakeen))} fill={b.s("METHEOR_OVERLAY_COLOR")} fillOpacity="1" stroke="transparent" />
          <path d={b.border(b.territoryOf(map.carthag))} fill={b.s("METHEOR_OVERLAY_COLOR")} fillOpacity="1" stroke="transparent" />
          <path d={b.border(map.imperialBasin)} fill={b.s("METHEOR_OVERLAY_COLOR")} fillOpacity="1" stroke="transparent" />
        </>
      )}
      {b.applicable("Ssw") && board.specialStrongholds.includes(map.shieldWall) && (
        <path d={b.border(map.shieldWall)} fill="#FF6F34" fillOpacity="0.6" stroke="transparent" />
      )}
      {board.highlightedTerritories.filter((t) => t !== hmsTerritory && !b.terr(t).isHomeworld).map((t) => (
        <path key={`h${t}`} d={b.border(t)} fill={b.s("HIGHLIGHT_OVERLAY_COLOR")} fillOpacity="1" stroke="red" strokeWidth="1" strokeLinecap="round" strokeLinejoin="round" />
      ))}
      {b.planetLocations().filter((l) => l.kind !== "hiddenMobileStronghold" && l.kind !== "discovered").map((l) => (
        <LocationShape key={l.id} b={b} id={l.id} click={click} />
      ))}
    </svg>
  );
}

// A clickable location of Planet.razor, with its intel popover.
function LocationShape({ b, id, click }: { b: Board; id: number; click: ReturnType<typeof useMapClick> }) {
  const { ref, attributes } = useMapPopover<SVGGElement>(locationIntel(b, id), { delay: 50 });
  return (
    <g clipPath={`url(#${sectorClipId(b.sector(id))})`}>
      <g ref={ref} fill={locationColor(b, id)} cursor="pointer" pointerEvents="all" data-location-id={id} {...attributes}
        onClick={(e) => click(id, e)}
        onContextMenu={(e) => { e.preventDefault(); click(id, e, true); }}>
        <path d={b.border(b.territoryOf(id))} />
      </g>
    </g>
  );
}

// MapHover.razor: a hidden outline per sector; hover.ts shows the one under the mouse.
export function MapHover() {
  const b = useBoard();
  const { X: w, Y: h } = b.point("MapDimensions");
  const locations = b.planetLocations().filter((l) => b.hasBorder(l.id));
  const territories = [...new Set(locations.map((l) => l.territory))];
  return (
    <svg className="maphover" data-sectors={SECTORS} width={w} height={h} viewBox={`0 0 ${w} ${h}`}
      style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%" }} pointerEvents="none">
      <defs>
        {territories.map((t) => <clipPath key={t} id={`maphover-t-${t}`}><path d={b.border(t)} /></clipPath>)}
      </defs>
      {locations.map((l) => {
        const border = b.border(l.territory);
        const sector = b.sector(l.id);
        const center = b.hasCenter(l.id) ? b.getCenter(l.id) : null;
        const edges = sector >= 0 ? wedgeEdges(b, sector) : null;
        return (
          <g key={l.id} className="hover-shape" data-location-id={l.id} data-territory-id={l.territory} data-sector={sector}
            data-cx={center ? num(center.X) : undefined} data-cy={center ? num(center.Y) : undefined}>
            <g clipPath={`url(#${sectorClipId(sector)})`}>
              <path d={border} className="fill" />
              <g clipPath={`url(#maphover-t-${l.territory})`}>
                <path d={border} className="core" />
                <path d={border} className="edge" />
                {edges && (
                  <>
                    <path d={edges[0]} className="core radial start" />
                    <path d={edges[1]} className="core radial end" />
                    <path d={edges[2]} className="core radial" />
                    <path d={edges[0]} className="edge radial start" />
                    <path d={edges[1]} className="edge radial end" />
                    <path d={edges[2]} className="edge radial" />
                  </>
                )}
              </g>
            </g>
          </g>
        );
      })}
    </svg>
  );
}

// RecentBlows.razor: tints the sectors spice blew in this turn, during the blow phase.
export function RecentBlows() {
  const b = useBoard();
  const piles = b.view.piles;
  if (b.view.mainPhase !== "Blow") return null;
  const blown = [piles.latestSpiceCardA, piles.latestSpiceCardB]
    .map((c) => (c == null ? null : b.skin.resourceCardInfo(c)?.location ?? null))
    .filter((l): l is number => l != null && b.loc(l).spiceBlowAmount > 0 && b.hasBorder(l));
  if (blown.length === 0) return null;
  const { X: w, Y: h } = b.point("MapDimensions");
  const territories = [...new Set(blown.map((l) => b.territoryOf(l)))];
  return (
    <svg className="mapblow" width={w} height={h} viewBox={`0 0 ${w} ${h}`}
      style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%" }} pointerEvents="none">
      <defs>
        {territories.map((t) => <clipPath key={t} id={`mapblow-t-${t}`}><path d={b.border(t)} /></clipPath>)}
      </defs>
      {blown.map((l, i) => {
        const border = b.border(b.territoryOf(l));
        const sector = b.sector(l);
        const clip = sectorClipId(sector);
        return (
          <Fragment key={i}>
            <g clipPath={`url(#${clip})`}>
              <path d={border} className="fill" />
              <path d={border} className="core" />
              <path d={border} className="edge" />
            </g>
            {sector >= 0 && (
              <g clipPath={`url(#mapblow-t-${b.territoryOf(l)})`}>
                <g clipPath={`url(#${clip})`}>
                  <path d={wedgePath(b, sector)} className="core radial" />
                  <path d={wedgePath(b, sector)} className="edge radial" />
                </g>
              </g>
            )}
          </Fragment>
        );
      })}
    </svg>
  );
}
