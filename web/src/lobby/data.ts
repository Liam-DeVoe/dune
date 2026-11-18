// The lobby data (web/public/static/lobby.json, written by rust/view/src/lobby.rs): every enum
// the lobby screens show, with its member names, numbers (which game settings in the savegame
// format use) and default skin descriptions, and the static tables of the lobby screens
// (available factions, rulesets, rule groups, ...).
import type { LobbyData } from "../generated/LobbyData";
import type { EnumMember } from "../generated/EnumMember";
import { withBase } from "../base";

export type { LobbyData };

export type LobbyEnum = "Faction" | "Ruleset" | "Rule" | "RuleGroup" | "MainPhase" | "Phase" | "UserStatus" | "SubscriptionType";

export class Lobby {
  private readonly byValue = new Map<string, Map<number, EnumMember>>();
  private readonly byName = new Map<string, Map<string, EnumMember>>();

  constructor(readonly data: LobbyData) {
    for (const [type, members] of Object.entries(data.enums)) {
      this.byValue.set(type, new Map(members!.map((m) => [m.value, m])));
      this.byName.set(type, new Map(members!.map((m) => [m.name, m])));
    }
  }

  /** `Skin.Describe` of an enum value given as its number. */
  label(type: LobbyEnum, value: number | null | undefined): string {
    if (value == null) return "";
    return this.byValue.get(type)?.get(value)?.label ?? String(value);
  }

  /** `Skin.Describe` of an enum value given as its name. */
  labelOf(type: LobbyEnum, name: string | null | undefined): string {
    if (name == null) return "";
    return this.byName.get(type)?.get(name)?.label ?? name;
  }

  /** The member name of a number (`Faction` 1 is "Atreides"). */
  name<T extends string = string>(type: LobbyEnum, value: number): T {
    return (this.byValue.get(type)?.get(value)?.name ?? String(value)) as T;
  }

  /** The number of a member name. */
  value(type: LobbyEnum, name: string): number {
    const v = this.byName.get(type)?.get(name)?.value;
    if (v === undefined) throw new Error(`Unknown ${type} ${name}`);
    return v;
  }

  /** Every member, in `Enum.GetValues` order. */
  members(type: LobbyEnum): EnumMember[] {
    return this.data.enums[type] ?? [];
  }
}

export async function loadLobby(): Promise<Lobby> {
  const r = await fetch(withBase("static/lobby.json"));
  if (!r.ok) throw new Error(`/static/lobby.json: ${r.status}`);
  return new Lobby((await r.json()) as LobbyData);
}
