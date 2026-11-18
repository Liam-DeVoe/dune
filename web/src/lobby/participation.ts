// The game's participation (who plays, observes and hosts), with the changes the server's
// notifications make to it (joinGame, observeGame, removeUser, setOrUnsetHost, openOrCloseSeats,
// assignSeats).
//
// The lobby screens list users in dictionary order (JoinedPlayersPanel, SeatPlayersComponent). A
// participation received whole (GameInitInfo, loadGame, assignSeats) comes as [key, value] pairs in
// the server's order, which is insertion order. The notifications keep that order here too: a
// removed user's place closes up and a new user goes last, as with the server's IndexMap and
// shift_remove (rust/server/src/participation.rs), so a reload doesn't reorder the list.
import type { IntDict, ParticipationInfo } from "./types";

/** A map from numbers, in insertion order. Setting a key that is there keeps its place. */
export class OrderedDict<V> {
  private map = new Map<number, V>();

  static from<V>(pairs: IntDict<V> | null | undefined): OrderedDict<V> {
    const d = new OrderedDict<V>();
    for (const [k, v] of pairs ?? []) d.set(k, v);
    return d;
  }

  get(key: number): V | undefined {
    return this.map.get(key);
  }

  has(key: number): boolean {
    return this.map.has(key);
  }

  set(key: number, value: V): void {
    this.map.set(key, value);
  }

  delete(key: number): void {
    this.map.delete(key);
  }

  keys(): number[] {
    return [...this.map.keys()];
  }

  values(): V[] {
    return [...this.map.values()];
  }

  items(): [number, V][] {
    return [...this.map.entries()];
  }

  get size(): number {
    return this.map.size;
  }

  toJson(): IntDict<V> {
    return this.items();
  }

  clone(): OrderedDict<V> {
    const d = new OrderedDict<V>();
    d.map = new Map(this.map);
    return d;
  }
}

export class Participation {
  playerNames: OrderedDict<string>;
  seatedPlayers: OrderedDict<number>;
  observers: Set<number>;
  hosts: Set<number>;
  availableSeats: Set<number>;
  kicked: Set<number>;
  botsSpeed: number;

  constructor(json: ParticipationInfo | null | undefined) {
    this.playerNames = OrderedDict.from(json?.playerNames);
    this.seatedPlayers = OrderedDict.from(json?.seatedPlayers);
    this.observers = new Set(json?.observers ?? []);
    this.hosts = new Set(json?.hosts ?? []);
    this.availableSeats = new Set(json?.availableSeats ?? []);
    this.kicked = new Set(json?.kicked ?? []);
    this.botsSpeed = json?.botsSpeed ?? 1;
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

  /** The name of a user. */
  playerName(userId: number): string {
    return this.playerNames.get(userId) ?? "?";
  }

  /** The user in a seat. */
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

  /** Whether a seat is open. */
  seatIsAvailable(seat: number): boolean {
    return this.availableSeats.has(seat);
  }

  /** The observers' names. */
  observerNames(): string[] {
    return this.playerNames.items().filter(([id]) => this.observers.has(id)).map(([, n]) => n);
  }

  // The server's notifications.

  /** joinGame. */
  addPlayer(userId: number, name: string, seat: number): void {
    this.seatedPlayers.set(userId, seat);
    this.playerNames.set(userId, name);
    this.availableSeats.delete(seat);
    this.observers.delete(userId);
  }

  /** observeGame. */
  addObserver(userId: number, name: string): void {
    this.observers.add(userId);
    this.playerNames.set(userId, name);
  }

  /** removeUser. */
  removeUser(userId: number, kick: boolean): void {
    this.seatedPlayers.delete(userId);
    this.observers.delete(userId);
    this.playerNames.delete(userId);
    this.hosts.delete(userId);
    if (kick) this.kicked.add(userId);
  }

  /** openOrCloseSeats, once per seat. */
  openOrCloseSeat(seat: number): void {
    if (this.availableSeats.has(seat)) this.availableSeats.delete(seat);
    else this.availableSeats.add(seat);
  }

  /** setOrUnsetHost. */
  setOrUnsetHost(userId: number): void {
    if (this.hosts.has(userId)) this.hosts.delete(userId);
    else this.hosts.add(userId);
  }

  /** assignSeats: the assignment replaces the seated players. */
  assignSeats(assignment: IntDict<number>): void {
    this.seatedPlayers = OrderedDict.from(assignment);
  }
}
