// What stands on the planet: MapComponents/ResourcesOnPlanet.razor, Monsters.razor, SSW.razor,
// ForcesOnPlanet.razor and ForcesInLocation.razor.
import type React from "react";
import { Fragment, useState } from "react";
import type { ForcesInLocation as Battalion } from "../../generated/ForcesInLocation";
import type { HeroRef } from "../../generated/HeroRef";
import { f, useBoard, type Board } from "./geometry";
import { Circle, MapForce, MapImage, MapLeader, MapText } from "./primitives";
import { locationIntel } from "./intel";
import { useMapClick } from "./mapInteraction";
import * as pop from "./popups";

// ResourcesOnPlanet.razor.
export function ResourcesOnPlanet() {
  const b = useBoard();
  const spice = new Map(b.view.board.spice.map((s) => [s.location, s.amount]));
  const r = b.resourceRadius;
  return (
    <>
      {b.planetLocations().filter((l) => l.spiceBlowAmount > 0).map((l) => {
        const at = b.spiceLocation(l.id);
        const amount = spice.get(l.id);
        if (amount !== undefined) {
          return (
            <Fragment key={l.id}>
              <MapImage x={at.X - r} y={at.Y - r} width={r * 2} height={r * 2} src={b.plainUrl("Harvester_URL")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} />
              <MapText toShow={String(amount)} x={at.X} y={at.Y} font={b.s("RESOURCETOKEN_FONT")} fontColor={b.s("RESOURCETOKEN_FONTCOLOR")}
                borderWidth={b.n("RESOURCETOKEN_FONT_BORDERWIDTH")} borderColor={b.s("RESOURCETOKEN_FONT_BORDERCOLOR")} align="center" />
            </Fragment>
          );
        }
        if (!b.d.DrawResourceIconsOnMap) return null;
        return (
          <Fragment key={l.id}>
            <MapImage x={at.X - r} y={at.Y - r} width={r * 2} height={r * 2} src={b.plainUrl("Resource_URL")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} />
            <MapText toShow={String(l.spiceBlowAmount)} x={at.X} y={at.Y} font={scaleFont(b.s("RESOURCETOKEN_FONT"), 0.7)} fontColor="#FFFFFFDD"
              borderWidth={b.n("RESOURCETOKEN_FONT_BORDERWIDTH")} borderColor="#646464DD" align="center" />
          </Fragment>
        );
      })}
    </>
  );
}

// Utilities.ScaleFont: every "<n>px" part of a font scaled, the font lower-cased.
export function scaleFont(font: string, scale: number): string {
  return font.toLowerCase().split(" ").map((part) => {
    if (!part.includes("px")) return " " + part;
    const n = Number(part.slice(0, part.indexOf("px")));
    return " " + (Number.isNaN(n) || part.slice(0, part.indexOf("px")) === "" ? part : `${f(scale * n)}px`);
  }).join("");
}

// Monsters.razor.
export function Monsters() {
  const b = useBoard();
  const r = b.monsterRadius;
  return (
    <>
      {b.view.board.monsters.map((m, i) => {
        const at = b.getCenter(b.terr(m.territory).middleLocation);
        return <MapImage key={i} x={at.X - r} y={at.Y - r} width={2 * r} height={2 * r} src={b.plainUrl("Monster_URL")} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} />;
      })}
    </>
  );
}

// SSW.razor: the worms on the Shield Wall.
export function Ssw() {
  const b = useBoard();
  const map = b.skin.statics.map;
  if (!b.applicable("Ssw") || b.view.board.specialStrongholds.includes(map.shieldWall)) return null;
  const at = b.getCenter(b.terr(map.shieldWall).middleLocation);
  const r = b.monsterRadius;
  const items = [];
  for (let i = 0; i < b.view.board.numberOfMonsters; i++) {
    const tip = `${natural(i)} ${b.skin.concept("Monster")}`;
    items.push(
      <MapImage key={i} x={i * f(f(0.7) * r) + at.X - f(f(0.35) * r)} y={at.Y} width={f(0.7 * r)} height={f(0.7 * r)} src={b.plainUrl("Monster_URL")}
        popover={tip} shadowColor="white" shadowX={1} shadowY={1} shadowBlur={1} />,
    );
  }
  return <>{items}</>;
}

// Game.Natural.
function natural(i: number): string {
  return ["First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth"][i] ?? String(i + 1);
}

function heroImage(b: Board, h: HeroRef): string {
  if (h.t === "leader") return b.personImage(h.id);
  if (h.t === "messiah") return b.plainUrl("Messiah_URL");
  return b.cardImage(h.id);
}

// ForcesOnPlanet.razor.
export function ForcesOnPlanet() {
  const b = useBoard();
  const click = useMapClick();
  const [showHms, setShowHms] = useState(true);
  const board = b.view.board;
  const me = b.view.viewer.faction;
  const map = b.skin.statics.map;
  const tr = b.tokenRadius;
  const fr = b.forceRadius;
  const owners = new Map(board.strongholds.map((s) => [s.location, s.owner]));
  const highlighted = new Set(board.highlightedTerritories);
  const out: React.ReactNode[] = [];

  for (const stronghold of b.strongholds()) {
    const center = b.getCenter(stronghold.id);
    const isHms = stronghold.id === map.hiddenMobileStronghold;
    const x = isHms ? center.X - f(1.6 * tr) : center.X - f(2.2 * tr);
    let y = center.Y - tr;
    const owner = owners.get(stronghold.id);
    if (owner !== undefined) {
      out.push(<MapImage key={`o${stronghold.id}`} x={x} y={y} diameter={tr} src={b.factionTableImage(owner)} opacity={isHms && !showHms ? f(0.2) : 1}
        popover={() => pop.strongholdAdvantage(b, b.terr(stronghold.territory).advantage, owner)} popoverStyle="" />);
      y += tr;
    }
    board.terror.filter((t) => t.territory === stronghold.territory).forEach((t, i) => {
      const known = me === "Moritani";
      out.push(<MapImage key={`t${stronghold.id}-${i}`} x={x} y={y} filter="hue-rotate(180deg);" diameter={tr} src={b.factionTableImage("Moritani")}
        shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0}
        popover={known && t.token ? () => pop.terror(b, t.token!) : "Terror"} popoverAsTooltip={!known} popoverStyle={known ? "" : undefined} />);
      y += tr;
    });
    const ambassador = board.ambassadors.find((a) => a.territory === stronghold.territory);
    if (ambassador) {
      out.push(<MapImage key={`a${stronghold.id}`} x={x} y={y} diameter={tr} src={b.url("AmbassadorImage_URL", ambassador.ambassador)}
        shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} popover={() => pop.ambassador(b, ambassador.ambassador)} popoverStyle="" />);
      y += tr;
    }
  }

  const forces = new Map<number, Battalion[]>();
  for (const bt of board.forces) {
    if (!forces.has(bt.location)) forces.set(bt.location, []);
    forces.get(bt.location)!.push(bt);
  }

  // OrderBy is a stable sort.
  const locations = b.planetLocations().filter((l) => b.visible(l.id))
    .map((l, i) => ({ l, i, y: b.getCenter(l.id).Y }))
    .sort((p, q) => p.y - q.y || p.i - q.i)
    .map((p) => p.l);

  for (const location of locations) {
    const pos = b.getCenter(location.id);
    let showForces = true;
    let forceDiameter = 2 * fr;
    const discovery = board.discoveries.find((d) => d.location === location.id);
    if (location.kind === "hiddenMobileStronghold") {
      if (!showHms) showForces = false;
      forceDiameter = f(1.5 * fr);
      const hr = b.hmsRadius;
      out.push(<MapImage key={`hms${location.id}`} x={pos.X - f(f(0.7) * hr)} y={pos.Y - hr} width={hr + Math.abs(b.hmsDx)} height={2 * hr} src={b.plainUrl("HMS_URL")}
        shadowColor={b.shadow} shadowX={5} shadowY={5} shadowBlur={1} opacity={showHms ? 1 : f(0.3)}
        popover={() => locationIntel(b, location.id)} onClick={(e) => click(location.id, e)} />);
      if (showHms && highlighted.has(location.territory)) {
        out.push(<Circle key={`hmsc${location.id}`} x={pos.X + f(f(0.3) * hr)} y={pos.Y} radius={f(f(0.85) * hr)} fillColor={b.s("HIGHLIGHT_OVERLAY_COLOR")} fillOpacity={1} strokeColor="red" strokeWidth={1} />);
      }
      out.push(<MapImage key={`hmse${location.id}`} x={pos.X - f(f(0.4) * hr) + Math.abs(b.hmsDx)} y={pos.Y - f(f(0.18) * hr)} diameter={fr}
        src={b.plainUrl(showHms ? "Eye_URL" : "EyeSlash_URL")} shadowColor="black" shadowX={1} shadowY={1} shadowBlur={0}
        onClick={() => setShowHms(!showHms)} popover={showHms ? "Hide HMS" : "Show HMS"} popoverAsTooltip />);
    } else if (location.kind === "discovered") {
      forceDiameter = f(1.5 * fr);
      const d = f(2.5 * fr);
      out.push(<MapImage key={`ds${location.id}`} x={pos.X - f(1.25 * fr)} y={pos.Y - f(1.25 * fr)} diameter={d}
        src={b.url("DiscoveryTokenImage_URL", location.discovery ?? "")} popover={() => locationIntel(b, location.id)} shadowX={1} shadowY={1}
        onClick={(e) => click(location.id, e)} />);
      if (highlighted.has(location.territory)) {
        out.push(<Circle key={`dsc${location.id}`} x={pos.X} y={pos.Y} radius={f(1.25 * fr)} fillColor={b.s("HIGHLIGHT_OVERLAY_COLOR")} fillOpacity={1} strokeColor="red" strokeWidth={1} />);
      }
    } else if (discovery) {
      const known = discovery.token != null;
      const d = f(2.5 * fr);
      out.push(<MapImage key={`dt${location.id}`} x={pos.X - f(1.25 * fr)} y={pos.Y - f(1.25 * fr)} diameter={d}
        src={b.url("DiscoveryTokenTypeImage_URL", discovery.tokenType)}
        popover={known ? () => pop.discovery(b, discovery.token!) : b.skin.get("DiscoveryTokenTypeName_STR", discovery.tokenType)}
        popoverAsTooltip={!known} popoverStyle={known ? "" : undefined} opacity={f(0.9)} shadowX={1} shadowY={1} />);
    }
    if (showForces) {
      out.push(<ForcesInLocation key={`f${location.id}`} location={location.id} battalions={forces} diameter={forceDiameter} />);
    }
  }
  return <>{out}</>;
}

// ForcesInLocation.razor.
function ForcesInLocation({ location, battalions, diameter }: { location: number; battalions: Map<number, Battalion[]>; diameter: number }) {
  const b = useBoard();
  const c = b.getCenter(location);
  const pos = { X: c.X - Math.trunc(f(0.5 * diameter)), Y: c.Y };
  const territory = b.territoryOf(location);
  let yOffset = 0;
  const out: React.ReactNode[] = [];
  const here = battalions.get(location);
  if (here) {
    yOffset = -f(0.5 * f(here.length * diameter));
    here.forEach((bt, i) => {
      let xOffset = 0;
      if (bt.forces > 0) {
        out.push(<MapForce key={`n${i}`} x={pos.X} y={yOffset + pos.Y} special={false} amount={bt.forces} faction={bt.faction} threatened={bt.threatened} showPopover={false} diameter={diameter} />);
        xOffset += diameter;
      }
      if (bt.specialForces > 0) {
        out.push(<MapForce key={`s${i}`} x={pos.X + xOffset} y={yOffset + pos.Y} special amount={bt.specialForces} faction={bt.faction} threatened={bt.threatened} showPopover={false} diameter={diameter} />);
      }
      yOffset += diameter;
    });
  }
  // Game.AnyForcesIn(territory): any battalion in any of its locations.
  const terr = b.terr(territory);
  const anyForces = terr.locations.some((l) => battalions.has(l));
  if (anyForces ? here !== undefined : location === terr.middleLocation) {
    let xOffset = 0;
    b.view.board.leaders.filter((h) => h.territory === territory).forEach((h, i) => {
      out.push(<MapLeader key={`l${i}`} src={heroImage(b, h.hero)} x={pos.X + xOffset} y={yOffset + pos.Y} diameter={diameter} popover={() => pop.hero(b, h.hero)} />);
      xOffset += diameter;
    });
  }
  return <>{out}</>;
}
