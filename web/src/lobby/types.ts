// The lobby's types: the protocol's (generated from rust/protocol into web/src/protocol), and
// the helpers the lobby screens use with them. Maps keyed by user id come as `[key, value]`
// pairs in the server's order (an object would come out of JSON.parse ordered by key).
import type { GameInfo } from "../protocol/GameInfo";

export type { Account } from "../protocol/Account";
export type { AdminInfo } from "../protocol/AdminInfo";
export type { AvailableSeat } from "../protocol/AvailableSeat";
export type { GameInfo };
export type { GameInitInfo } from "../protocol/GameInitInfo";
export type { GameListScope } from "../protocol/GameListScope";
export type { Lobby as LobbyStatus } from "../protocol/Lobby";
export type { ParticipationInfo } from "../protocol/ParticipationInfo";
export type { PhaseControlInfo } from "../protocol/PhaseControlInfo";
export type { ScheduledGameInfo } from "../protocol/ScheduledGameInfo";
export type { SeenUser } from "../protocol/SeenUser";
export type { ServerInfo } from "../protocol/ServerInfo";
export type { SubscriptionType } from "../protocol/SubscriptionType";
export type { UserStatus } from "../protocol/UserStatus";

/** A map keyed by user id or seat on the wire: `[key, value]` pairs in the server's order. */
export type IntDict<V> = Array<[number, V]>;

/** The value of `key` in an `IntDict`. */
export function dictGet<V>(d: IntDict<V>, key: number): V | undefined {
  return d.find(([k]) => k === key)?.[1];
}

/** `GameSettings` in savegame format, as `updateSettings` takes them and `EstablishPlayers`
 * carries them. */
export interface GameSettings {
  NumberOfPlayers: number;
  MaximumTurns: number;
  InitialRules: number[];
  AllowedFactionsInPlay: number[];
  AutoOpenEmptySeats: boolean;
  AsyncPlay: boolean;
  AsyncPlayMessageIntervalMinutes: number;
}

/** `new GameSettings()`: the settings of a game until its host sends theirs. */
export function defaultSettings(): GameSettings {
  return { NumberOfPlayers: 6, MaximumTurns: 10, InitialRules: [], AllowedFactionsInPlay: [], AutoOpenEmptySeats: true, AsyncPlay: false, AsyncPlayMessageIntervalMinutes: 0 };
}

/** `GameInfo.CanBeJoined`: users join without a seat while the game awaits players and has
 * room, and on a seat a host opened at any time. */
export function canBeJoined(g: GameInfo): boolean {
  return (g.phase === "AwaitingPlayers" && g.seatedPlayers.length < g.maxPlayers) || g.availableSeats.length > 0;
}

/** `GameInfo.YouAreIn`. */
export function youAreIn(g: GameInfo, userId: number): boolean {
  return g.seatedPlayers.some(([k]) => k === userId);
}

/** `GameInfo.YourCurrentSeat`. */
export function yourCurrentSeat(g: GameInfo, userId: number): number {
  return dictGet(g.seatedPlayers, userId) ?? -1;
}
