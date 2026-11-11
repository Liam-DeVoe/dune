// OtherComponents/Log/StormLog.razor: a ring of the board's 18 sectors with the stretch the storm
// swept in red and the distance in the middle, the dials under it, and one box per place the
// storm hit with each loss on its own line. Flips, Weather Control and Family Atomics are asides
// above the ring. Hovering a box lights its sectors on the ring (and on the map), and hovering a
// sector lights the boxes in it. The "storm will move" and "storm moves" lines are left out
// because the ring shows them. Every other line keeps its place.
import { Fragment, type ReactNode, useState } from "react";
import type { Faction } from "../../generated/enums";
import type { LogLine } from "../../generated/LogLine";
import type { Part } from "../../generated/Part";
import type { Skin } from "../../skin";
import { useGame } from "../../state/context";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { PlaceChip } from "../PlaceChip";
import { SimpleNumber } from "../SimpleNumber";
import { SpiceComponent } from "../SpiceComponent";
import { forceName, PartText, type PhaseLogProps, placeId, rec, type RecordOf, samePart } from "./common";

const SECTORS = 18;

// Ring geometry, in the SVG's 200x200 viewBox.
const Cx = 100, Cy = 100, Outer = 92, Inner = 57;

// d.ToString("0.##", InvariantCulture): at most two decimals, rounded half away from zero.
function N(d: number): string {
  const r = Math.sign(d) * Math.round(Math.abs(d) * 100) / 100;
  return Object.is(r, -0) ? "-0" : String(r);
}

// Sector s spans angles (6 - s) to (5 - s) times 20 degrees, as on the map (SectorGeometry).
function angle(sector: number): number {
  return 2 * Math.PI * (6 - sector) / SECTORS;
}

function P(a: number, r: number): string {
  return `${N(Cx + r * Math.cos(a))},${N(Cy + r * Math.sin(a))}`;
}

function wedge(sector: number): string {
  const a0 = angle(sector), a1 = angle(sector + 1);
  return `M${P(a0, Inner)} L${P(a0, Outer)} A${N(Outer)},${N(Outer)} 0 0 0 ${P(a1, Outer)} L${P(a1, Inner)} A${N(Inner)},${N(Inner)} 0 0 1 ${P(a0, Inner)}Z`;
}

// An arc inside the ring along the swept sectors, in the storm's direction.
function arrow(from: number, distance: number): string {
  const r = Inner - 9;
  const a0 = angle(from + 1) - 0.08, a1 = angle(from + distance + 1) + 0.14;
  const large = distance * 20 > 180 ? 1 : 0;
  return `M${P(a0, r)} A${N(r)},${N(r)} 0 ${large} 0 ${P(a1, r)}`;
}

type Loss = RecordOf<"StormKilled"> | RecordOf<"StormDestroyedSpice">;

// One box per territory the storm hit: the locations hit, in order, and each loss in the game's order.
interface Place { territory: number; locations: Part[]; losses: Loss[]; sectors: Set<number> }

function places(skin: Skin, lines: LogLine[]): Place[] {
  const result: Place[] = [];
  for (const l of lines) {
    const data = l.record;
    if (data?.type !== "StormKilled" && data?.type !== "StormDestroyedSpice") continue;
    const location = data.location;
    const info = skin.locationInfo(placeId(location));
    const territory = info?.territory ?? -1;
    let place = result.find((p) => p.territory === territory);
    if (!place) {
      place = { territory, locations: [], losses: [], sectors: new Set() };
      result.push(place);
    }
    if (!place.locations.some((x) => samePart(x, location))) {
      place.locations.push(location);
      place.sectors.add(info?.sector ?? -1);
    }
    place.losses.push(data);
  }
  return result;
}

// "[faction] 3 [force] 1 [special force]": a count and an icon per kind.
function Forces({ faction, forces, specialForces }: { faction: Faction; forces: number; specialForces: number }) {
  return (
    <>
      <FactionChip faction={faction} />
      {forces > 0 && <span className="storm-forces"><SimpleNumber>{forces}</SimpleNumber><ForceToken faction={faction} size={20} /></span>}
      {specialForces > 0 && <span className="storm-forces"><SimpleNumber>{specialForces}</SimpleNumber><ForceToken faction={faction} special size={20} /></span>}
    </>
  );
}

// A card as a pill with its art, as the log shows treachery cards.
function CardPill({ card, type }: { card: Part | null; type: string }) {
  const { skin } = useGame();
  return card == null ? <b>{skin.enumValue("TreacheryCardType", type)}</b> : <PartText part={card} />;
}

export function StormLog({ lines, line }: PhaseLogProps) {
  const { skin } = useGame();
  // Sectors lit by hovering a box or a sector, and the box hovered, if any.
  const [hot, setHot] = useState<{ sectors: Set<number>; place: number | null }>({ sectors: new Set(), place: null });
  const light = (sectors: Set<number>, place: number | null = null) => setHot({ sectors, place });

  const anchor = lines.findIndex((l) => l.record?.type === "StormMoved" || l.record?.type === "StormPlaced");
  const drawn = anchor >= 0;
  const anchorMoved = drawn ? rec(lines[anchor], "StormMoved") : null;
  const weatherCard = anchorMoved?.card ?? null;
  const shieldWall = skin.statics.map.shieldWall;
  const shieldWallLocations = skin.territoryInfo(shieldWall)?.locations ?? [];

  const ring = (data: LogLine["record"]): ReactNode => {
    let from: number | null = null;
    let end: number, distance = 0;
    if (data?.type === "StormMoved") {
      from = data.from;
      distance = data.distance;
      end = (data.from + data.distance) % SECTORS;
    } else {
      end = data?.type === "StormPlaced" ? data.sector : 0;
    }
    const swept = new Set(Array.from({ length: distance }, (_, d) => (from! + d + 1) % SECTORS));
    const bySector = new Map<number, number[]>();
    for (const l of skin.statics.locations) {
      if (l.kind === "homeworld" || l.sector < 0) continue;
      if (!bySector.has(l.sector)) bySector.set(l.sector, []);
      bySector.get(l.sector)!.push(l.id);
    }
    return (
      <svg className="storm-ring" viewBox="0 0 200 200" width="170" height="170" role="img"
        aria-label={from == null ? `The storm is in sector ${end + 1}` : `The storm moves ${distance} sectors`}>
        <defs>
          <marker id="storm-ring-head" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="4" markerHeight="4" orient="auto-start-reverse">
            <path d="M0 0 L10 5 L0 10z" />
          </marker>
        </defs>
        {Array.from({ length: SECTORS }, (_, sector) => {
          const state = sector === end ? "end" : swept.has(sector) ? "swept" : sector === from ? "start" : "";
          const h = hot.sectors.has(sector) ? "hot" : "";
          return (
            <path key={sector} className={`w ${state} ${h}`} d={wedge(sector)} data-ref-locations={(bySector.get(sector) ?? []).join(",")}
              onMouseEnter={() => light(new Set([sector]))} onMouseLeave={() => light(new Set())} />
          );
        })}
        <circle className="rim" cx={N(Cx)} cy={N(Cy)} r={N(Outer + 1.5)} />
        {from != null && distance > 1 && <path className="arrow" d={arrow(from, distance)} markerEnd="url(#storm-ring-head)" />}
        {from != null && (
          <>
            <text className="distance" x={N(Cx)} y={N(Cy + 6)} textAnchor="middle">{distance}</text>
            <text className="label" x={N(Cx)} y={N(Cy + 24)} textAnchor="middle">SECTORS</text>
          </>
        )}
      </svg>
    );
  };

  const dialsLog = lines.map((l) => rec(l, "StormDials")).filter((d) => d != null).pop();
  const dials = dialsLog && (
    <div className="storm-dials">
      <FactionChip faction={dialsLog.first} /><SimpleNumber>{dialsLog.firstDial}</SimpleNumber>
      <span className="plus">+</span>
      <FactionChip faction={dialsLog.second} /><SimpleNumber>{dialsLog.secondDial}</SimpleNumber>
    </div>
  );

  const boxes = places(skin, lines).map((place, i) => {
    const overlaps = [...place.sectors].some((s) => hot.sectors.has(s));
    const h = (hot.place != null ? hot.place === place.territory : overlaps) ? "hot" : "";
    const chip = place.locations.length === 1 ? { location: placeId(place.locations[0]) } : { territory: place.territory };
    return (
      <div key={`box${i}`} className={`log-box storm-place ${h}`} data-ref-locations={place.locations.map(placeId).join(",")}
        onMouseEnter={() => light(place.sectors, place.territory)} onMouseLeave={() => light(new Set())}>
        <PlaceChip place={chip} />
        <div className="lines">
          {place.losses.map((loss, j) => loss.type === "StormKilled"
            ? <div key={j}>The storm kills <Forces faction={loss.faction} forces={loss.forces} specialForces={loss.specialForces} /></div>
            : <div key={j}>The storm destroys <SpiceComponent amount={String(loss.amount)} /></div>)}
        </div>
      </div>
    );
  });

  return (
    <>
      {lines.map((message, i) => {
        const data = message.record;
        switch (data?.type) {
          case "BeneGesseritFlipped":
            return (
              <div key={i} className="log-aside">
                <FactionChip faction="BeneGesserit" /> are alone and flip to <ForceToken faction="BeneGesserit" size={20} title={forceName(skin, "BeneGesserit", false)} /> in <PlaceChip place={{ territory: placeId(data.territory) }} />
              </div>
            );
          case "FamilyAtomicsUsed": {
            const killed = lines.slice(i + 1).map((l) => rec(l, "FamilyAtomicsKilled")).filter((k) => k != null);
            return (
              <Fragment key={i}>
                <div className="log-aside">
                  <FactionChip faction={data.faction} /> use <CardPill card={data.card} type="Metheor" /> to destroy the <PlaceChip place={{ territory: shieldWall }} />!
                </div>
                {killed.length > 0 && (
                  <div className="log-box" data-ref-locations={shieldWallLocations.join(",")}>
                    <PlaceChip place={{ territory: shieldWall }} />
                    <div className="lines">
                      {killed.map((k, j) => (
                        <div key={j}>{skin.enumValue("TreacheryCardType", "Metheor")} kills <Forces faction={k.faction} forces={k.forces} specialForces={k.specialForces} /></div>
                      ))}
                    </div>
                  </div>
                )}
              </Fragment>
            );
          }
          case "FamilyAtomicsKilled":
            return null;
          case "StormForecast":
          case "StormDials":
          case "StormKilled":
          case "StormDestroyedSpice":
            if (drawn) return null;
            break;
          case "StormMoved":
          case "StormPlaced":
            if (i === anchor) {
              const weather = data.type === "StormMoved" && data.by !== "None" ? data : null;
              return (
                <Fragment key={i}>
                  {weather && (
                    <div className="log-aside">
                      <FactionChip faction={weather.by} /> use <CardPill card={weather.card} type="StormSpell" /> to move the storm <SimpleNumber>{weather.distance}</SimpleNumber> sectors
                    </div>
                  )}
                  {ring(data)}
                  {dials}
                  {boxes}
                </Fragment>
              );
            }
            break;
        }
        if (weatherCard != null && message.fact?.kind === "Discard" && samePart(message.fact.card, weatherCard)) return null;
        return <Fragment key={i}>{line(message)}</Fragment>;
      })}
    </>
  );
}
