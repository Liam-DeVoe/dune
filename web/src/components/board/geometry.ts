// The board's model: the current view, the skin and the static map data, with the geometry the
// C# map components share: Skin.GetCenter, GetSpiceLocation, GetTerritoryBorder, and
// MapComponents/SectorGeometry.cs. Every map layer reads the board through this, so the layers
// stay close to their razor files.
import { createContext, useContext } from "react";
import type { Faction, Rule } from "../../generated/enums";
import type { GameView } from "../../generated/GameView";
import type { LocationInfo } from "../../generated/LocationInfo";
import type { TerritoryInfo } from "../../generated/TerritoryInfo";
import type { Point, Skin } from "../../skin";

export type { Point };

// C# float arithmetic, where rounding to single precision can matter (casts to int).
export const f = Math.fround;

// Map.NumberOfSectors.
export const SECTORS = 18;

// MapComponent.TwoPi (a float constant).
export const TWO_PI = f(6.2831853072);

// MapComponent.Px (Utilities.Px): a length in map units for an inline style.
export function px(x: number): string {
  return x > -0.001 && x < 0.001 ? "0" : `${x}px`;
}

// Utilities.Round: the number as .NET prints a double.
export function num(x: number): string {
  return String(x);
}

type Dict<T = string> = Record<string, T>;

export class Board {
  readonly locations: LocationInfo[];
  private readonly locationById: Map<number, LocationInfo>;
  private readonly territoryById: Map<number, TerritoryInfo>;
  private readonly attached: Map<number, { attachedTo: number; sector: number }>;
  private readonly rules: Set<Rule>;

  constructor(readonly view: GameView, readonly skin: Skin) {
    const s = skin.statics;
    this.locations = s.locations;
    this.locationById = new Map(s.locations.map((l) => [l.id, l]));
    this.territoryById = new Map(s.territories.map((t) => [t.id, t]));
    this.attached = new Map(view.board.attached.map((a) => [a.location, a]));
    this.rules = new Set([...view.rules, ...view.rulesForBots]);
  }

  // Skin values.
  get d(): Dict<unknown> { return this.skin.data as Dict<unknown>; }
  n(key: string): number { return this.d[key] as number; }
  s(key: string): string { return (this.d[key] as string | undefined) ?? ""; }
  point(key: string): Point { return this.d[key] as Point; }
  // Skin.GetLabel for a URL: "" when missing, as in C#.
  url(dict: string, key: string | number): string { return this.skin.url((this.d[dict] as Dict | undefined)?.[String(key)]); }
  plainUrl(key: string): string { return this.skin.url(this.s(key)); }

  get mapW(): number { return this.point("MapDimensions").X; }
  get mapH(): number { return this.point("MapDimensions").Y; }
  get center(): Point { return this.point("PlanetCenter"); }
  get planetRadius(): number { return this.n("PlanetRadius"); }
  get mapRadius(): number { return this.n("MapRadius"); }
  get tokenRadius(): number { return this.n("PlayerTokenRadius"); }
  get forceRadius(): number { return this.n("FORCETOKEN_RADIUS"); }
  get resourceRadius(): number { return this.n("RESOURCETOKEN_RADIUS"); }
  get monsterRadius(): number { return this.n("MONSTERTOKEN_RADIUS"); }
  get shadow(): string { return this.s("SHADOW"); }
  // Skin.HmsDx and HmsRadius.
  get hmsDx(): number { return f(-3 * this.tokenRadius); }
  get hmsRadius(): number { return f(1.5 * this.tokenRadius); }

  applicable(rule: Rule): boolean { return this.rules.has(rule); }

  loc(id: number): LocationInfo { return this.locationById.get(id)!; }
  terr(id: number): TerritoryInfo { return this.territoryById.get(id)!; }
  territoryOf(location: number): number { return this.loc(location).territory; }
  isHms(id: number): boolean { return this.loc(id)?.kind === "hiddenMobileStronghold"; }
  isDiscovered(id: number): boolean { return this.loc(id)?.kind === "discovered"; }
  isHomeworld(id: number): boolean { return this.loc(id)?.kind === "homeworld"; }

  // Map.Locations(false), in map order.
  planetLocations(): LocationInfo[] { return this.locations.filter((l) => l.kind !== "homeworld"); }

  // Map.Strongholds.
  strongholds(): LocationInfo[] { return this.locations.filter((l) => this.terr(l.territory).isStronghold); }

  // Location.Visible: attached locations only once attached.
  visible(id: number): boolean {
    const l = this.loc(id);
    return l.kind === "hiddenMobileStronghold" || l.kind === "discovered" ? this.attached.has(id) : true;
  }

  // Location.Sector; attached locations take their host's.
  sector(id: number): number {
    return this.attached.get(id)?.sector ?? this.loc(id).sector;
  }

  border(territory: number): string { return this.skin.get("TerritoryBorder_SVG", territory); }

  // HasBorder in the map layers.
  hasBorder(id: number): boolean {
    const l = this.loc(id);
    return l.kind !== "hiddenMobileStronghold" && l.kind !== "discovered" && l.kind !== "homeworld" && this.border(l.territory) !== "";
  }

  // SectorGeometry.HasCenter.
  hasCenter(id: number): boolean {
    const l = this.loc(id);
    return l.kind === "hiddenMobileStronghold" || l.kind === "discovered" || (this.d.LocationCenter_Point as Dict<Point>)[String(id)] !== undefined;
  }

  // Skin.GetCenter.
  getCenter(id: number): Point {
    const l = this.loc(id);
    if (l.kind === "hiddenMobileStronghold") {
      const a = this.attached.get(id);
      if (a) {
        const rock = this.territoryOf(a.attachedTo) === this.skin.statics.map.rockOutcroppings;
        const c = this.getCenter(a.attachedTo);
        return { X: c.X + Math.trunc(this.hmsDx) + (rock ? 22 : 0), Y: c.Y + (rock ? -10 : 0) };
      }
      return { X: Math.trunc(this.hmsDx), Y: Math.trunc(this.hmsDx) };
    }
    if (l.kind === "discovered") {
      const a = this.attached.get(id);
      if (a) {
        const c = this.getCenter(a.attachedTo);
        return { X: c.X - f(1.5 * this.forceRadius), Y: c.Y - f(1.5 * this.forceRadius) };
      }
      return { X: -4 * this.forceRadius, Y: -4 * this.forceRadius };
    }
    return (this.d.LocationCenter_Point as Dict<Point>)[String(id)];
  }

  // Skin.GetSpiceLocation.
  spiceLocation(id: number): Point {
    return this.loc(id).spiceBlowAmount !== 0 ? (this.d.LocationSpice_Point as Dict<Point>)[String(id)] : { X: 0, Y: 0 };
  }

  // SectorGeometry.PlayerTokenPosition.
  playerTokenPosition(seat: number): Point {
    const positionSector = Math.floor(f(seat * SECTORS / this.view.maximumPlayers));
    const angle = (4.5 - positionSector) * 2 * Math.PI / SECTORS;
    return {
      X: this.center.X + f(this.mapRadius * Math.cos(angle)),
      Y: this.center.Y + f(this.mapRadius * Math.sin(angle)),
    };
  }

  factionColor(faction: Faction): string { return (this.d.FactionColor as Dict)[faction] ?? ""; }
  factionTableImage(faction: Faction): string { return this.url("FactionTableImage_URL", faction); }
  factionFaceDownImage(faction: Faction): string { return this.url("FactionFacedownImage_URL", faction); }
  forceImage(faction: Faction, special: boolean): string {
    return this.url(special ? "FactionSpecialForceImage_URL" : "FactionForceImage_URL", faction);
  }
  personImage(id: number): string { return this.url("PersonImage_URL", id); }
  cardImage(id: number): string { return this.url("TreacheryCardImage_URL", this.skin.cardSkinId(id)); }
  resourceCardImage(skinId: number): string { return this.url("ResourceCardImage_URL", skinId); }
}

export const BoardContext = createContext<Board | null>(null);

export function useBoard(): Board {
  const b = useContext(BoardContext);
  if (!b) throw new Error("useBoard outside the board");
  return b;
}

// SectorGeometry: the outline of one storm sector, in the coordinates of the territory borders.
function wedgePoints(b: Board, sector: number) {
  const scaleX = 563 / b.mapW;
  const scaleY = 626 / b.mapH;
  const r = b.planetRadius;
  const { X: cx, Y: cy } = b.center;
  const startX = Math.cos(2 * Math.PI * (6 - sector) / SECTORS) * r + cx;
  const startY = Math.sin(2 * Math.PI * (6 - sector) / SECTORS) * r + cy;
  const endX = Math.cos(2 * Math.PI * (5 - sector) / SECTORS) * r + cx;
  const endY = Math.sin(2 * Math.PI * (5 - sector) / SECTORS) * r + cy;
  return {
    c: `${num(scaleX * cx)},${num(scaleY * cy)}`,
    s: `${num(scaleX * startX)},${num(scaleY * startY)}`,
    e: `${num(scaleX * endX)},${num(scaleY * endY)}`,
    r: `${num(scaleX * r)} ${num(scaleY * r)}`,
    angle: num(360 / SECTORS),
  };
}

export function wedgePath(b: Board, sector: number): string {
  const w = wedgePoints(b, sector);
  return `M ${w.c} L ${w.s} A ${w.r} ${w.angle} 0 0 ${w.e} Z`;
}

export function wedgeEdges(b: Board, sector: number): [string, string, string] {
  const w = wedgePoints(b, sector);
  return [`M ${w.c} L ${w.s}`, `M ${w.c} L ${w.e}`, `M ${w.s} A ${w.r} ${w.angle} 0 0 ${w.e}`];
}

export function sectorClipId(sector: number): string {
  return "clipOfSector" + sector;
}
