// The lobby payloads of rust/docs/protocol.md (Payload objects), as the server sends them: C#
// member names, enums as numbers, int-keyed dictionaries as arrays of [key, value] pairs in the
// server's order (an object would come out of JSON.parse ordered by key).
// LoginInfo, GameInitInfo and CallResult are in state/connection.ts.

/** `Dictionary<int, V>` on the wire: `[key, value]` pairs in the dictionary's order. */
export type IntDict<V> = Array<[number, V]>;

/** The value of `key` in an `IntDict`. */
export function dictGet<V>(d: IntDict<V>, key: number): V | undefined {
  return d.find(([k]) => k === key)?.[1];
}

/** `AvailableSeatInfo`. */
export interface AvailableSeatInfo {
  Seat: number;
  Faction: number;
  IsBot: boolean;
}

/** `GameInfo` (Shared/Events/GameInfo.cs): one game in the lobby. */
export interface GameInfo {
  CreatorId: number;
  CreatorUsername: string;
  CreationDate: string;
  GameId: string;
  HasPassword: boolean;
  Name: string;
  MaxPlayers: number;
  MaxTurns: number;
  MainPhase: number;
  Phase: number;
  Turn: number;
  NrOfBots: number;
  NrOfPlayers: number;
  FactionsInPlay: number[] | null;
  Ruleset: number;
  LastActivity: string | null;
  SeatedPlayers: IntDict<number>;
  AvailableSeats: AvailableSeatInfo[];
}

/** `ScheduledGameInfo`. */
export interface ScheduledGameInfo {
  ScheduledGameId: string;
  DateTime: string;
  CreatorUserId: number;
  CreatorName: string;
  NumberOfPlayers: number | null;
  MaximumTurns: number | null;
  Ruleset: number | null;
  AsyncPlay: boolean;
  AllowedFactionsInPlay: number[];
  Subscribers: IntDict<number>;
  SubscriberNames: IntDict<string>;
}

/** `LoggedInUserInfo`. */
export interface LoggedInUserInfo {
  Id: number;
  Name: string;
  Status: number;
  LastSeen: string | null;
}

/** `ServerStatus`: the lobby. */
export interface ServerStatus {
  RunningGames: GameInfo[];
  OwnGames: GameInfo[];
  ScheduledGames: ScheduledGameInfo[];
  RecentlySeenUsers: LoggedInUserInfo[];
}

/** `ServerInfo`. */
export interface ServerInfo {
  ScheduledMaintenance: string;
  AdminName: string;
}

/** `UserInfo`, one row of `AdminInfo.Users`. */
export interface UserInfo {
  Id: number;
  Username: string;
  Email: string;
  LastLogin: string;
  PlayerName: string;
}

/** `AdminInfo`. */
export interface AdminInfo {
  Users: UserInfo[];
  UsersByUserTokenCount: number;
  ConnectionInfoByUserIdCount: number;
  GamesByGameIdCount: number;
}

/** `Participation` (Shared/Model/Participation.cs). */
export interface ParticipationJson {
  PlayerNames: IntDict<string>;
  SeatedPlayers: IntDict<number>;
  Observers: number[];
  Hosts: number[];
  AvailableSeats: number[];
  Kicked: number[];
  BotsSpeed: number;
}

/** `PhaseControlInfo`. */
export interface PhaseControlInfo {
  Active: boolean;
  WaitingFor: number[];
  Countdown: number;
  CountdownEnds: string | null;
  Holding: number[];
}

/** `GameSettings` in savegame format. */
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

/** `GameListScope`. */
export const GameListScope = { None: 1, Active: 2, All: 3 } as const;

/** `GameInfo.CanBeJoined`. `phaseAwaiting` is `Phase == AwaitingPlayers`. */
export function canBeJoined(g: GameInfo, phaseAwaiting: boolean): boolean {
  return (phaseAwaiting && g.SeatedPlayers.length < g.MaxPlayers) || g.AvailableSeats.length > 0;
}

/** `GameInfo.YouAreIn`. */
export function youAreIn(g: GameInfo, userId: number): boolean {
  return g.SeatedPlayers.some(([k]) => k === userId);
}

/** `GameInfo.YourCurrentSeat`. */
export function yourCurrentSeat(g: GameInfo, userId: number): number {
  return dictGet(g.SeatedPlayers, userId) ?? -1;
}
