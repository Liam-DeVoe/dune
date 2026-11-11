// The game's Participation as the C# client kept it (Shared/Model/Participation.cs), with the
// changes Game_Participation.cs makes when the hub's notifications arrive (JoinGame, ObserveGame,
// RemoveUser, SetOrUnsetHost, OpenOrCloseSeats, AssignSeats).
//
// The lobby screens list users in dictionary order (JoinedPlayersPanel, SeatPlayersComponent), so
// the dictionaries keep .NET's Dictionary order: insertion order, except that a key added after a
// removal takes the removed key's slot. A participation received whole (GameInitInfo, LoadGame,
// AssignSeats) comes as [key, value] pairs in the server's order. Known limit: the server's
// dictionary may have a free slot from an earlier removal, which the pairs don't show, so the
// next user to join is listed last here and in that slot on the server.
import type { IntDict, ParticipationJson } from "./types";

/** A .NET `Dictionary<int, V>`: enumeration order with free-slot reuse. */
export class NetDict<V> {
  private entries: ([number, V] | null)[] = [];
  private free: number[] = [];

  static from<V>(pairs: IntDict<V> | null | undefined): NetDict<V> {
    const d = new NetDict<V>();
    for (const [k, v] of pairs ?? []) d.set(k, v);
    return d;
  }

  private index(key: number): number {
    return this.entries.findIndex((e) => e !== null && e[0] === key);
  }

  get(key: number): V | undefined {
    const i = this.index(key);
    return i < 0 ? undefined : this.entries[i]![1];
  }

  has(key: number): boolean {
    return this.index(key) >= 0;
  }

  set(key: number, value: V): void {
    const i = this.index(key);
    if (i >= 0) this.entries[i] = [key, value];
    else if (this.free.length > 0) this.entries[this.free.pop()!] = [key, value];
    else this.entries.push([key, value]);
  }

  delete(key: number): void {
    const i = this.index(key);
    if (i < 0) return;
    this.entries[i] = null;
    this.free.push(i);
  }

  keys(): number[] {
    return this.entries.filter((e) => e !== null).map((e) => e![0]);
  }

  values(): V[] {
    return this.entries.filter((e) => e !== null).map((e) => e![1]);
  }

  items(): [number, V][] {
    return this.entries.filter((e) => e !== null).map((e) => [e![0], e![1]]);
  }

  get size(): number {
    return this.keys().length;
  }

  toJson(): IntDict<V> {
    return this.items();
  }

  clone(): NetDict<V> {
    const d = new NetDict<V>();
    d.entries = this.entries.map((e) => (e ? [e[0], e[1]] : null));
    d.free = [...this.free];
    return d;
  }
}

export class Participation {
  playerNames: NetDict<string>;
  seatedPlayers: NetDict<number>;
  observers: Set<number>;
  hosts: Set<number>;
  availableSeats: Set<number>;
  kicked: Set<number>;
  botsSpeed: number;

  constructor(json: ParticipationJson | null | undefined) {
    this.playerNames = NetDict.from(json?.PlayerNames);
    this.seatedPlayers = NetDict.from(json?.SeatedPlayers);
    this.observers = new Set(json?.Observers ?? []);
    this.hosts = new Set(json?.Hosts ?? []);
    this.availableSeats = new Set(json?.AvailableSeats ?? []);
    this.kicked = new Set(json?.Kicked ?? []);
    this.botsSpeed = json?.BotsSpeed ?? 1;
  }

  clone(): Participation {
    const p = new Participation(null);
    p.playerNames = this.playerNames.clone();
    p.seatedPlayers = this.seatedPlayers.clone();
    p.observers = new Set(this.observers);
    p.hosts = new Set(this.hosts);
    p.availableSeats = new Set(this.availableSeats);
    p.kicked = new Set(this.kicked);
    p.botsSpeed = this.botsSpeed;
    return p;
  }

  /** `Game.GetPlayerName(int userId)`. */
  playerName(userId: number): string {
    return this.playerNames.get(userId) ?? "?";
  }

  /** `Game.UserIdInSeat`. */
  userIdInSeat(seat: number): number {
    for (const [userId, s] of this.seatedPlayers.items()) if (s === seat) return userId;
    return -1;
  }

  isHost(userId: number | null | undefined): boolean {
    return userId != null && this.hosts.has(userId);
  }

  isObserver(userId: number | null | undefined): boolean {
    return userId != null && this.observers.has(userId);
  }

  /** `Game.SeatIsAvailable`. */
  seatIsAvailable(seat: number): boolean {
    return this.availableSeats.has(seat);
  }

  /** `Game.ObserverNames`. */
  observerNames(): string[] {
    return this.playerNames.items().filter(([id]) => this.observers.has(id)).map(([, n]) => n);
  }

  // The notification handlers (Client.cs Handle*, through Game_Participation.cs).

  /** `Game.AddPlayer` (HandleJoinGame). */
  addPlayer(userId: number, name: string, seat: number): void {
    this.seatedPlayers.set(userId, seat);
    this.playerNames.set(userId, name);
    this.availableSeats.delete(seat);
    this.observers.delete(userId);
  }

  /** `Game.AddObserver` (HandleObserveGame). */
  addObserver(userId: number, name: string): void {
    this.observers.add(userId);
    this.playerNames.set(userId, name);
  }

  /** `Game.RemoveUser` (HandleRemoveUser). */
  removeUser(userId: number, kick: boolean): void {
    this.seatedPlayers.delete(userId);
    this.observers.delete(userId);
    this.playerNames.delete(userId);
    this.hosts.delete(userId);
    if (kick) this.kicked.add(userId);
  }

  /** `Game.OpenOrCloseSeat` (HandleOpenOrCloseSeats, once per seat). */
  openOrCloseSeat(seat: number): void {
    if (this.availableSeats.has(seat)) this.availableSeats.delete(seat);
    else this.availableSeats.add(seat);
  }

  /** `Game.SetOrUnsetHost` (HandleSetOrUnsetHost). */
  setOrUnsetHost(userId: number): void {
    if (this.hosts.has(userId)) this.hosts.delete(userId);
    else this.hosts.add(userId);
  }

  /** HandleAssignSeats: `Participation.SeatedPlayers = assignment`. */
  assignSeats(assignment: IntDict<number>): void {
    this.seatedPlayers = NetDict.from(assignment);
  }
}
