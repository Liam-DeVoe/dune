// The skin: names, images and colours, read from the C# DefaultSkin exported as JSON
// (tools/export-skin.cs), plus the engine's static data (rust/view/examples/static_data.rs).
// A port of the parts of Shared/Model/Skin.cs that turn ids and enum values into text and URLs.
// The view never carries names or URLs; everything visual comes from here, so a custom skin
// (RequestSetSkin) is a different JSON file and nothing else.
import type { Faction, TreacheryCardType } from "./generated/enums";
import type { StaticData } from "./generated/StaticData";
import { BASE, withBase } from "./base";

type Dict = Record<string, string>;

export interface SkinData {
  FactionName_STR: Dict;
  FactionImage_URL: Dict;
  FactionColor: Dict;
  TerritoryName_STR: Dict;
  TreacheryCardName_STR: Dict;
  TreacheryCardImage_URL: Dict;
  PersonName_STR: Dict;
  PersonImage_URL: Dict;
  Concept_STR: Dict;
  MainPhase_STR: Dict;
  [key: string]: unknown;
}

// The engine's static data (rust/view/src/static_data.rs); its type is generated.
export type { StaticData };

export interface Point { X: number; Y: number }

export class Skin {
  private readonly locations: Map<number, StaticData["locations"][number]>;
  private readonly cards: Map<number, StaticData["treacheryCards"][number]>;
  private readonly resourceCards: Map<number, StaticData["resourceCards"][number]>;
  private readonly leaders: Map<number, StaticData["leaders"][number]>;
  private readonly territories: Map<number, StaticData["territories"][number]>;

  constructor(readonly data: SkinData, readonly statics: StaticData) {
    this.locations = new Map(statics.locations.map((l) => [l.id, l]));
    this.cards = new Map(statics.treacheryCards.map((c) => [c.id, c]));
    this.resourceCards = new Map(statics.resourceCards.map((c) => [c.skinId, c]));
    this.leaders = new Map(statics.leaders.map((l) => [l.id, l]));
    this.territories = new Map(statics.territories.map((t) => [t.id, t]));
  }

  // A display fallback for names: the key when the skin has no entry.
  private label(dict: Dict | undefined, key: string | number): string {
    return dict?.[String(key)] ?? String(key);
  }

  // Skin.GetLabel: the entry, or "" when it is missing, as in C#. Popover markup uses this.
  get(dict: string, key: string | number): string {
    return (this.data[dict] as Dict | undefined)?.[String(key)] ?? "";
  }

  // Skin URLs start with "." (DefaultSkin.DEFAULT_ART_LOCATION); the app serves wwwroot at its base
  // path. Markup built as strings (popovers) keeps the skin's URL as C# does; index.html's
  // <base href> (the base path) resolves it.
  url(u: string | undefined): string {
    return u ? u.replace(/^\.\//, BASE) : "";
  }

  faction(f: Faction): string { return this.label(this.data.FactionName_STR, f); }
  factionImage(f: Faction): string { return this.url(this.data.FactionImage_URL[f]); }
  factionColor(f: Faction): string { return this.data.FactionColor[f] ?? "#646464"; }
  territory(id: number): string { return this.label(this.data.TerritoryName_STR, id); }
  // Treachery cards are keyed by SkinId in the skin and by Id in the view.
  cardSkinId(id: number): number { return this.cards.get(id)?.skinId ?? id; }
  cardType(id: number): TreacheryCardType { return this.cards.get(id)?.cardType ?? "None"; }
  card(id: number): string { return this.label(this.data.TreacheryCardName_STR, this.cardSkinId(id)); }
  cardImage(id: number): string { return this.url(this.data.TreacheryCardImage_URL[String(this.cardSkinId(id))]); }
  person(id: number): string { return this.label(this.data.PersonName_STR, id); }
  personImage(id: number): string { return this.url(this.data.PersonImage_URL[String(id)]); }
  leaderFaction(id: number): Faction { return this.leaders.get(id)?.faction ?? "None"; }
  concept(c: string): string { return this.get("Concept_STR", c); }
  str(key: string): string { return (this.data[key] as string | undefined) ?? ""; }

  locationInfo(id: number) { return this.locations.get(id); }
  territoryInfo(id: number) { return this.territories.get(id); }
  resourceCardInfo(skinId: number) { return this.resourceCards.get(skinId); }
  // GetTerritoryBorder.
  territoryBorder(id: number): string { return this.get("TerritoryBorder_SVG", id); }
  get mapDimensions(): Point { return this.data.MapDimensions as Point; }
  get planetCenter(): Point { return this.data.PlanetCenter as Point; }
  get planetRadius(): number { return this.data.PlanetRadius as number; }

  // Skin.Describe(Location): "<territory> (<orientation> Sector)", or just the territory.
  location(id: number): string {
    const l = this.locations.get(id);
    if (!l) return `Location ${id}`;
    const t = this.territory(l.territory);
    return l.orientation ? `${t} (${l.orientation} Sector)` : t;
  }

  // Skin.Describe(object) for enum values: the type's own dictionary when the skin has one
  // (Concept_STR, MainPhase_STR, TreacheryCardType_STR, ...), the member name otherwise.
  // Text the game writes arrives described by the server; this is for the client's own labels.
  enumValue(type: string, value: string): string {
    const dict = this.data[`${type}_STR`] as Dict | undefined;
    return dict?.[value] ?? value;
  }
}

export async function loadSkin(): Promise<Skin> {
  const [data, statics] = await Promise.all([
    fetch(withBase("skin/default.json")).then((r) => r.json() as Promise<SkinData>),
    fetch(withBase("static/data.json")).then((r) => r.json() as Promise<StaticData>),
  ]);
  return new Skin(data, statics);
}
