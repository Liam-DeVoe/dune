// The game's participation (who plays, observes and hosts), with the changes the server's
// notifications make to it (joinGame, observeGame, removeUser, setOrUnsetHost, openOrCloseSeats,
// assignSeats).
//
// The lobby screens list users in dictionary order (JoinedPlayersPanel, SeatPlayersComponent). A
// participation received whole (GameInitInfo, loadGame, assignSeats) comes as [key, value] pairs in
// the server's order, which is insertion order.
// Known quirk (known-issues.md): SlotDict gives a key added after a removal the removed key's
// slot, where the server (rust/server/src/participation.rs, IndexMap with shift_remove) lists it
// last. So a user who joins after someone left can be listed in a different place after a reload.
import type { IntDict, ParticipationInfo } from "./types";

/** A map from numbers, in insertion order, except that a key added after a removal takes the
 * removed key's slot. */
export class SlotDict<V> {
  private entries: ([number, V] | null)[] = [];
  private free: number[] = [];

  static from<V>(pairs: IntDict<V> | null | undefined): SlotDict<V> {
    const d = new SlotDict<V>();
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

  clone(): SlotDict<V> {
    const d = new SlotDict<V>();
    d.entries = this.entries.map((e) => (e ? [e[0], e[1]] : null));
    d.free = [...this.free];
    return d;
  }
}

export class Participation {
  playerNames: SlotDict<string>;
  seatedPlayers: SlotDict<number>;
  observers: Set<number>;
  hosts: Set<number>;
  availableSeats: Set<number>;
  kicked: Set<number>;
  botsSpeed: number;

  constructor(json: ParticipationInfo | null | undefined) {
    this.playerNames = SlotDict.from(json?.playerNames);
    this.seatedPlayers = SlotDict.from(json?.seatedPlayers);
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
    this.seatedPlayers = SlotDict.from(assignment);
  }
}
