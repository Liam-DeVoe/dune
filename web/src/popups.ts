// Markup the C# client builds as strings: the popovers of Skin.GetPopup (Shared/Model/Skin.cs)
// and of Client/Support.cs, and the place chip (Support.PlaceChipHtml with
// MapComponents/RegionGeometry and SectorGeometry). Bootstrap shows popovers from an HTML
// string (data-bs-content), so these return strings, built exactly as C# builds them.
// rust/skin/src/skin.rs is the checked port of the Skin half.
import type { Ambassador, DiscoveryToken, Faction, LeaderSkill, StrongholdAdvantage, TechToken, TerrorType, TreacheryCardType } from "./generated/enums";
import type { Skin } from "./skin";
import { htmlEncode, replaceSpiceAmounts, supportRound, utilitiesRound } from "./text";

// Skin.SpiceHtml: a spice amount drawn on the harvester icon.
export function spiceHtml(skin: Skin, amount: string, size = 18): string {
  const name = skin.concept("Resource");
  return `<span class='spice-amt' style='--s:${size}px' title='${name}'><img src='${skin.str("Harvester_URL")}' alt='${name}'/><b>${amount}</b></span>`;
}

// Skin.WithSpiceIcons: the text as HTML, with each amount of spice in it drawn by spiceHtml.
export function withSpiceIcons(skin: Skin, text: string, size = 18, encode = true): string {
  if (text === "") return "";
  const t = encode ? htmlEncode(text) : text;
  return replaceSpiceAmounts(t, skin.concept("Resource"), (digits) => spiceHtml(skin, digits, size));
}

// Skin.FactionChipHtml.
export function factionChipHtml(skin: Skin, f: Faction, you = false, locationIds: number[] | null = null): string {
  const name = htmlEncode(skin.get("FactionName_STR", f));
  const locations = locationIds ? locationIds.join(",") : "";
  return `<span class='chip faction${you ? " you" : ""}' data-ref-faction='${f}' data-ref-locations='${locations}' style='--fc:${skin.data.FactionColor[f] ?? ""}' title='${name}'><img src='${skin.get("FactionImage_URL", f)}' alt=''/><span class='name'>${name}</span></span>`;
}

const framedCard = (url: string) => `<div class='pop-art'><img src='${url}'/></div>`;
const imageHover = (url: string) => `<img src='${url}' width=300 class='img-fluid' style='filter:drop-shadow(-3px 3px 2px black);'/>`;

function tokenPopup(skin: Skin, imageUrl: string | null, name: string, rulesText: string): string {
  const art = imageUrl ? `<div class='pop-token-art'><img src='${imageUrl}'/></div>` : "";
  return `<div class='pop-token'>${art}<div class='pop-token-body'><h6>${name}</h6><p>${withSpiceIcons(skin, rulesText)}</p></div></div>`;
}

// Skin.GetTreacheryCardCategory.
function cardCategory(t: TreacheryCardType): [string, string] {
  switch (t) {
    case "Projectile": return ["weapon", "Projectile weapon"];
    case "Poison": return ["weapon", "Poison weapon"];
    case "ProjectileAndPoison": return ["weapon", "Projectile and poison weapon"];
    case "Laser": case "WeirdingWay": case "PoisonTooth": case "ArtilleryStrike": case "MirrorWeapon": case "Rockmelter":
      return ["weapon", "Special weapon"];
    case "Shield": case "ProjectileDefense": return ["defense", "Projectile defense"];
    case "Antidote": case "PoisonDefense": case "PortableAntidote": return ["defense", "Poison defense"];
    case "ShieldAndAntidote": return ["defense", "Projectile and poison defense"];
    case "Chemistry": return ["defense", "Special defense"];
    case "Useless": return ["worthless", "Worthless"];
    default: return ["special", "Special"];
  }
}

// Skin.GetTreacheryCardRulesText: the description without its "Weapon - Type - " or
// "<name> - " prefix.
function cardRulesText(skin: Skin, id: number): string {
  const sid = skin.cardSkinId(id);
  const text = skin.get("TreacheryCardDescription_STR", sid);
  for (const prefix of ["Weapon - ", "Defense - "]) {
    if (!text.startsWith(prefix)) continue;
    const end = text.indexOf(" - ", prefix.length);
    return end >= 0 ? text.slice(end + 3) : text;
  }
  for (const prefix of ["Worthless Card - ", skin.get("TreacheryCardName_STR", sid) + " - "]) {
    if (text.startsWith(prefix)) return text.slice(prefix.length);
  }
  return text;
}

// Skin.GetPopup(TreacheryCard).
export function treacheryCardPopup(skin: Skin, id: number): string {
  const sid = skin.cardSkinId(id);
  const [category, type] = cardCategory(skin.cardType(id));
  return `<div class='pop-card'><img class='pop-card-art' src='${skin.get("TreacheryCardImage_URL", sid)}'/><div class='pop-card-body'><h6>${skin.get("TreacheryCardName_STR", sid)}</h6><div class='pop-card-type'><i class='${category}'></i>${type}</div><p>${withSpiceIcons(skin, cardRulesText(skin, id))}</p></div></div>`;
}

// Skin.GetPopup(IHero, Game) for a leader: the skill card with the leader on it when the leader
// has a skill, else the leader's portrait with its name and faction chip (GetLeaderPopup).
export function leaderPopup(skin: Skin, id: number, skill: LeaderSkill | null): string {
  const image = skin.get("PersonImage_URL", id);
  if (skill) {
    return `<div style='position:relative'><img style='position:relative;filter:drop-shadow(-3px 3px 2px black);' src='${skin.get("LeaderSkillCardImage_URL", skill)}' width=300/><img src='${image}' width=140 style='position:absolute;left:200px;top:120px;filter:drop-shadow(-2px 2px 2px black);'/></div>`;
  }
  const faction = skin.leaderFaction(id);
  return `<div class='pop-leader'><div class='pop-leader-art' style='--fc:${skin.data.FactionColor[faction] ?? ""}'><img src='${image}'/></div><div class='pop-leader-body'><h6>${skin.get("PersonName_STR", id)}</h6>${factionChipHtml(skin, faction)}</div></div>`;
}

// Skin.GetPopup(IHero) for the Messiah.
export function messiahPopup(skin: Skin): string {
  return imageHover(skin.str("Messiah_URL"));
}

// Skin.GetPopup(LeaderSkill).
export function leaderSkillPopup(skin: Skin, s: LeaderSkill): string {
  return framedCard(skin.get("LeaderSkillCardImage_URL", s));
}

// Skin.GetPopup(StrongholdAdvantage): the stronghold card.
export function strongholdAdvantagePopup(skin: Skin, adv: StrongholdAdvantage): string {
  return imageHover(skin.get("StrongholdCardImage_URL", adv));
}

// Skin.GetPopup(TechToken).
export function techTokenPopup(skin: Skin, t: TechToken): string {
  return tokenPopup(skin, skin.get("TechTokenImage_URL", t), skin.get("TechTokenName_STR", t), skin.get("TechTokenDescription_STR", t));
}

// Skin.GetPopup(TerrorType).
export function terrorPopup(skin: Skin, t: TerrorType): string {
  return tokenPopup(skin, null, skin.get("TerrorTokenName_STR", t), skin.get("TerrorTokenDescription_STR", t));
}

// Skin.GetPopup(Ambassador).
export function ambassadorPopup(skin: Skin, a: Ambassador): string {
  let text = skin.get("AmbassadorDescription_STR", a);
  const dash = text.indexOf(" - ");
  if (dash >= 0) text = text.slice(dash + 3);
  return tokenPopup(skin, skin.get("AmbassadorImage_URL", a), `${skin.get("AmbassadorName_STR", a)} Ambassador`, text);
}

// Skin.GetPopup(DiscoveryToken).
export function discoveryTokenPopup(skin: Skin, d: DiscoveryToken): string {
  return tokenPopup(skin, skin.get("DiscoveryTokenImage_URL", d), skin.get("DiscoveryTokenName_STR", d), skin.get("DiscoveryTokenDescription_STR", d));
}

// Client Support.GetPopup(ResourceCard): the card, with the place chip of its territory and
// its spice amount under it; worm cards are the card alone.
export function resourceCardPopup(skin: Skin, skinId: number): string {
  let html = `<div class="pop-spice"><img src="${skin.get("ResourceCardImage_URL", skinId)}"/>`;
  const location = skin.resourceCardInfo(skinId)?.location;
  if (location != null) {
    const amount = skin.locationInfo(location)?.spiceBlowAmount ?? 0;
    html += `<div class="pop-spice-foot">${placeChipHtml(skin, { location })}${spiceHtml(skin, String(amount), 26)}</div>`;
  }
  return html + "</div>";
}

// ---- The place chip ----

const ReferenceWidth = 563;
const ReferenceHeight = 626;
const NumberOfSectors = 18;
const Samples = 40;
const PlaceGlyphMinimumSize = 0.2;

function planetCenter(skin: Skin): [number, number] {
  return [skin.planetCenter.X * ReferenceWidth / skin.mapDimensions.X, skin.planetCenter.Y * ReferenceHeight / skin.mapDimensions.Y];
}

function planetRadius(skin: Skin): number {
  return skin.planetRadius * ReferenceWidth / skin.mapDimensions.X;
}

// SectorGeometry.WedgePath.
export function wedgePath(skin: Skin, sector: number): string {
  const scaleX = ReferenceWidth / skin.mapDimensions.X;
  const scaleY = ReferenceHeight / skin.mapDimensions.Y;
  const angle = Math.fround(360 / NumberOfSectors);
  const r = skin.planetRadius;
  const cx = skin.planetCenter.X;
  const cy = skin.planetCenter.Y;
  const startX = Math.cos(2 * Math.PI * (6 - sector) / NumberOfSectors) * r + cx;
  const startY = Math.sin(2 * Math.PI * (6 - sector) / NumberOfSectors) * r + cy;
  const endX = Math.cos(2 * Math.PI * (5 - sector) / NumberOfSectors) * r + cx;
  const endY = Math.sin(2 * Math.PI * (5 - sector) / NumberOfSectors) * r + cy;
  const R = utilitiesRound;
  return `M ${R(scaleX * cx)},${R(scaleY * cy)} ` +
    `L ${R(scaleX * startX)},${R(scaleY * startY)} ` +
    `A ${R(scaleX * r)} ${R(scaleY * r)} ${angle} 0 0 ${R(scaleX * endX)},${R(scaleY * endY)} Z`;
}

// RegionGeometry.Flatten: absolute M, L, C and Z, each curve as eight segments.
function flatten(path: string): [number, number][] {
  const points: [number, number][] = [];
  const numbers: number[] = [];
  let command = " ";
  const run = () => {
    if (command === "M" || command === "L") {
      for (let i = 0; i + 1 < numbers.length; i += 2) points.push([numbers[i], numbers[i + 1]]);
    } else if (command === "C") {
      for (let i = 0; i + 5 < numbers.length; i += 6) {
        const [x0, y0] = points.length > 0 ? points[points.length - 1] : [numbers[i], numbers[i + 1]];
        for (let k = 1; k <= 8; k++) {
          const t = k / 8;
          const u = 1 - t;
          points.push([
            u * u * u * x0 + 3 * u * u * t * numbers[i] + 3 * u * t * t * numbers[i + 2] + t * t * t * numbers[i + 4],
            u * u * u * y0 + 3 * u * u * t * numbers[i + 1] + 3 * u * t * t * numbers[i + 3] + t * t * t * numbers[i + 5],
          ]);
        }
      }
    }
    numbers.length = 0;
  };
  for (let t of path.replace(/,/g, " ").split(" ").filter((x) => x.length > 0)) {
    if (/\p{L}/u.test(t[0])) {
      run();
      command = t[0].toUpperCase();
      t = t.slice(1);
      if (t.length === 0) continue;
    }
    const v = Number(t);
    if (t.trim() !== "" && !Number.isNaN(v)) numbers.push(v);
  }
  run();
  return points;
}

function contains(polygon: [number, number][], x: number, y: number): boolean {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const [xi, yi] = polygon[i];
    const [xj, yj] = polygon[j];
    if ((yi > y) !== (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

function sectorOf(dx: number, dy: number): number {
  const angle = Math.atan2(dy, dx);
  const sector = Math.floor(6 - angle * NumberOfSectors / (2 * Math.PI));
  return ((sector % NumberOfSectors) + NumberOfSectors) % NumberOfSectors;
}

interface Piece { size: number; cx: number; cy: number }
const pieces = new Map<string, Piece>();

// RegionGeometry.Measure: the size (side of a square of the same area) and centre of mass of
// the part of an outline inside a sector, by sampling.
function measure(skin: Skin, path: string, sector: number): Piece {
  const key = `${sector}|${path}`;
  const cached = pieces.get(key);
  if (cached) return cached;
  const polygon = flatten(path);
  const xs = polygon.map((p) => p[0]);
  const ys = polygon.map((p) => p[1]);
  const [minX, minY, maxX, maxY] = [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
  const step = Math.max(maxX - minX, maxY - minY) / Samples;
  const [cx, cy] = planetCenter(skin);
  let inside = 0;
  let sumX = 0;
  let sumY = 0;
  for (let x = minX + step / 2; x < maxX; x += step) {
    for (let y = minY + step / 2; y < maxY; y += step) {
      if (!contains(polygon, x, y)) continue;
      if (sector >= 0 && sectorOf(x - cx, y - cy) !== sector) continue;
      inside++;
      sumX += x;
      sumY += y;
    }
  }
  // C# stores the centre in a PointD of floats.
  const piece = inside === 0
    ? { size: 0, cx: Math.fround((minX + maxX) / 2), cy: Math.fround((minY + maxY) / 2) }
    : { size: Math.sqrt(inside) * step, cx: Math.fround(sumX / inside), cy: Math.fround(sumY / inside) };
  pieces.set(key, piece);
  return piece;
}

let placeChipCounter = 0;

export type Place = { location: number; sector?: number } | { territory: number };

export interface PlaceChipData {
  id: string;
  locations: number[];
  title: string;
  view: string;
  cx: string;
  cy: string;
  r: string;
  wedge: string | null;
  border: string;
  transform: string;
  name: string;
  orientation: string;
}

// What Support.PlaceChipHtml computes. `sector` overrides the static sector for attached
// locations, whose host decides it.
export function placeChipData(skin: Skin, place: Place): PlaceChipData {
  const id = "place" + ++placeChipCounter;
  const loc = "location" in place ? skin.locationInfo(place.location) : undefined;
  const territory = "location" in place ? loc?.territory ?? -1 : place.territory;
  const R = supportRound;
  const [cx, cy] = planetCenter(skin);
  const r = planetRadius(skin);
  const sector = "location" in place ? (place.sector ?? loc?.sector ?? -1) : -1;
  const border = skin.territoryBorder(territory);
  let transform = "";
  if (border !== "") {
    const piece = measure(skin, border, sector);
    const grow = piece.size > 0 ? Math.max(1, PlaceGlyphMinimumSize * 2 * r / piece.size) : 1;
    transform = `translate(${R(piece.cx)},${R(piece.cy)}) scale(${R(grow)}) translate(${R(-piece.cx)},${R(-piece.cy)})`;
  }
  return {
    id,
    locations: "location" in place ? [place.location] : skin.territoryInfo(territory)?.locations ?? [],
    title: "location" in place ? skin.location(place.location) : skin.territory(territory),
    view: `${R(cx - r - 3)} ${R(cy - r - 3)} ${R(2 * r + 6)} ${R(2 * r + 6)}`,
    cx: R(cx),
    cy: R(cy),
    r: R(r),
    wedge: sector >= 0 ? wedgePath(skin, sector) : null,
    border,
    transform,
    name: skin.territory(territory),
    orientation: loc?.orientation ?? "",
  };
}

// Support.PlaceChipHtml: a pill with a glyph of the planet showing the place's outline where it
// lies, the territory name, and for a sector its compass word. PlaceChip.tsx draws the same.
export function placeChipHtml(skin: Skin, place: Place): string {
  const c = placeChipData(skin, place);
  let html = `<span class="chip place" data-ref-locations="${c.locations.join(",")}" title="${c.title}">`;
  html += `<svg class="wedge" viewBox="${c.view}" aria-hidden="true"><defs>`;
  html += `<clipPath id="${c.id}-disc"><circle cx="${c.cx}" cy="${c.cy}" r="${c.r}"/></clipPath>`;
  if (c.wedge !== null) html += `<clipPath id="${c.id}-sector"><path d="${c.wedge}"/></clipPath>`;
  html += `</defs><g clip-path="url(#${c.id}-disc)"><circle class="disc" cx="${c.cx}" cy="${c.cy}" r="${c.r}"/>`;
  if (c.border !== "") {
    const clip = c.wedge !== null ? ` clip-path="url(#${c.id}-sector)"` : "";
    html += `<g transform="${c.transform}"><g${clip}><path class="w" d="${c.border}"/></g></g>`;
  }
  html += `</g><circle class="ring" cx="${c.cx}" cy="${c.cy}" r="${c.r}" vector-effect="non-scaling-stroke"/></svg>`;
  html += `<span class="name">${c.name}</span>`;
  if (c.orientation !== "") html += `<span class="sector">${c.orientation}</span>`;
  return html + "</span>";
}
