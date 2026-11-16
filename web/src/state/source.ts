// Where views come from. The game screen renders whatever the current source holds and never
// computes game rules itself (rust/docs/client.md, State).
//
// - StaticSource: a view saved as JSON (web/public/views), for the skeleton and component tests.
// - WebSocketSource: a game on the Rust server. It gets a view after every event and every
//   participation change, and answers draft queries and submissions over the protocol in
//   rust/docs/protocol.md.
//
// Events, in drafts and submissions, are in the savegame format: PascalCase members, enums as
// numbers (FactionValue and friends in generated/enums.ts). The server sets `Initiator` to the
// faction of the authenticated user, so the client leaves it out.
import type { DraftResult } from "../generated/DraftResult";
import type { GameView } from "../generated/GameView";
import type { ReportLines } from "../generated/ReportLines";
import type { Call } from "../protocol/Call";
import type { GameInitInfo } from "../protocol/GameInitInfo";
import type { UndoEntry } from "../protocol/UndoEntry";
import { type CallError, type CallName, type Connection, type ConnectionStatus, failed, type Reply } from "./connection";

export type { UndoEntry };

export type GameEventDraft = Record<string, unknown> & { $type: string };

/** The calls about one game: those whose arguments name the game. */
export type GameCallName = { [C in CallName]: Extract<Call, { call: C }> extends { args: { game: string } } ? C : never }[CallName];

/** A game call's arguments other than the game. */
export type GameCallArgs<C extends GameCallName> = Omit<Extract<Call, { call: C }> extends { args: infer A } ? A : never, "game">;

export interface ViewSource {
  load(): Promise<GameView>;
  /** Later views, for sources whose game moves on. Returns an unsubscribe function. */
  subscribe?(onView: (view: GameView) => void): () => void;
  /** A draft query: the form's inputs as an event in savegame JSON. */
  query(draft: GameEventDraft): Promise<DraftResult>;
  /** Submits an event. Resolves with the server's error text, or null when it was accepted. */
  submit(event: GameEventDraft): Promise<string | null>;
  /** `report`: the lines of one report of `log.reports`, for the log's history. */
  report?(index: number): Promise<ReportLines>;
  /** `undoList`, for hosts (the undo panel). */
  undoList?(): Promise<UndoEntry[]>;
  /** `undoPreview`, for hosts: the view at gap `gap` (just before event `gap`), without
   * actions, for the undo timeline's preview. */
  undoPreview?(gap: number): Promise<GameView>;
  /** `saveGame`, for hosts and the admin: the live game as a savegame. */
  saveGame?(): Promise<string>;
  /** A call about this game (`undo`, `setBotSpeed`, `chat`, ...), with its arguments other than
   * the game. Resolves with the server's error text, or null when it succeeded. */
  gameCall?<C extends GameCallName>(name: C, args: GameCallArgs<C>): Promise<string | null>;
  /** The server connection's status (`Client.IsConnected`), for the board's Disconnection
   * banner. A source without a server has none and counts as connected. */
  connectionStatus?(): ConnectionStatus;
  /** Runs when `connectionStatus` changes. Returns an unsubscribe function. */
  onConnectionStatus?(handler: (s: ConnectionStatus) => void): () => void;
}

export class StaticSource implements ViewSource {
  constructor(private readonly url: string) {}

  async load(): Promise<GameView> {
    const r = await fetch(this.url);
    if (!r.ok) throw new Error(`${this.url}: ${r.status}`);
    return (await r.json()) as GameView;
  }

  // Without a server, a draft keeps the view's options and is never validated again.
  async query(draft: GameEventDraft): Promise<DraftResult> {
    const view = await this.load();
    const action = view.actions.find((a) => a.eventType === draft["$type"]);
    return { eventType: String(draft["$type"]), options: action?.options ?? null, validation: null };
  }

  async submit(): Promise<string | null> {
    return "This is a saved view; there is no server to send the action to.";
  }

  private reports: Promise<ReportLines[]> | null = null;

  /** The lines of every report, saved beside the view as `<view>.reports.json`
   * (rust/view/examples/scene_views.rs), so the log's history works without a server. */
  async report(index: number): Promise<ReportLines> {
    if (this.reports == null) {
      const url = this.url.replace(/\.json$/, ".reports.json");
      this.reports = fetch(url).then((r) => {
        if (!r.ok) throw new Error(`${url}: ${r.status}`);
        return r.json() as Promise<ReportLines[]>;
      });
      this.reports.catch(() => { this.reports = null; });
    }
    const all = await this.reports;
    const lines = all.find((r) => r.index === index);
    if (!lines) throw new Error(`no report ${index}`);
    return lines;
  }
}

function errorText(err: CallError): string {
  return err.message || err.code;
}

/** Heartbeats keep the user in the server's user list and run its housekeeping (protocol.md). */
const HEARTBEAT_MS = 5000;

export class WebSocketSource implements ViewSource {
  private view: GameView | null = null;
  private readonly listeners = new Set<(view: GameView) => void>();
  private readonly rejoinListeners = new Set<(info: GameInitInfo | null) => void>();
  private first: Promise<GameView>;
  private resolveFirst!: (view: GameView) => void;
  private readonly unsubscribe: (() => void)[] = [];
  private heartbeat: number | undefined;

  /** `conn` must be authenticated, or have a session to authenticate with
   * (`Connection.setSession`). */
  constructor(private readonly conn: Connection, readonly gameId: string) {
    this.first = new Promise((resolve) => { this.resolveFirst = resolve; });
    this.unsubscribe.push(conn.on("gameView", (v) => this.receive(v)));
    // After a reconnection the socket is a new connection outside the game's group: rejoin it.
    this.unsubscribe.push(conn.onOpen(() => { if (this.view) this.rejoin().catch(() => {}); }));
  }

  private receive(view: GameView): void {
    this.view = view;
    this.resolveFirst(view);
    this.listeners.forEach((l) => l(view));
  }

  private async reconnect(): Promise<Reply<"reconnectGame">> {
    await this.conn.invoke("serverInfo");
    // The connection authenticated the socket when it opened; reconnectGame adds it to the
    // game and sends the view.
    return this.conn.call("reconnectGame", { game: this.gameId });
  }

  private async join(): Promise<void> {
    const r = await this.reconnect();
    if (failed(r)) throw new Error(`reconnectGame: ${errorText(r.err)}`);
  }

  private async rejoin(): Promise<void> {
    const r = await this.reconnect();
    if (!failed(r)) this.rejoinListeners.forEach((l) => l(r.ok));
    // The user left or was kicked while the socket was down, or the game was closed.
    else if (r.err.code === "userNotInGame" || r.err.code === "gameNotFound") this.rejoinListeners.forEach((l) => l(null));
  }

  /** Joins the game's group and resolves with the first view. */
  async load(): Promise<GameView> {
    if (this.view) return this.view;
    await this.conn.open();
    await this.join();
    if (this.heartbeat === undefined) {
      this.heartbeat = window.setInterval(() => {
        this.conn.call("heartbeat", { scope: "None" }).catch(() => {});
      }, HEARTBEAT_MS);
    }
    return this.first;
  }

  subscribe(onView: (view: GameView) => void): () => void {
    this.listeners.add(onView);
    return () => this.listeners.delete(onView);
  }

  /** Runs each time the source rejoins the game on a new socket, with the game's participation
   * as the server has it now: changes made while the socket was down sent this tab no
   * notification. Runs with null when the server refused because the user is no longer in the
   * game, or the game is gone. Returns an unsubscribe function. */
  onRejoin(handler: (info: GameInitInfo | null) => void): () => void {
    this.rejoinListeners.add(handler);
    return () => this.rejoinListeners.delete(handler);
  }

  async query(draft: GameEventDraft): Promise<DraftResult> {
    const r = await this.conn.call("queryDraft", { game: this.gameId, eventNumber: this.view?.eventNumber ?? 0, draft });
    if (!failed(r)) return r.ok;
    // A refused query leaves the form's options and shows why, as a validation message.
    const action = this.view?.actions.find((a) => a.eventType === draft.$type);
    return { eventType: draft.$type, options: action?.options ?? null, validation: [{ t: "text", v: errorText(r.err) }] };
  }

  async submit(event: GameEventDraft): Promise<string | null> {
    try {
      const r = await this.conn.call("submitEvent", { game: this.gameId, event });
      return failed(r) ? errorText(r.err) : null;
    } catch (e) {
      return String(e);
    }
  }

  /** `report`: the lines of an earlier report. */
  report(index: number): Promise<ReportLines> {
    return this.conn.invoke("report", { game: this.gameId, index });
  }

  /** `undoList`, for hosts. */
  undoList(): Promise<UndoEntry[]> {
    return this.conn.invoke("undoList", { game: this.gameId });
  }

  /** `undoPreview`, for hosts. */
  undoPreview(gap: number): Promise<GameView> {
    return this.conn.invoke("undoPreview", { game: this.gameId, gap });
  }

  /** `saveGame`, for hosts and the admin. */
  saveGame(): Promise<string> {
    return this.conn.invoke("saveGame", { game: this.gameId });
  }

  async gameCall<C extends GameCallName>(name: C, args: GameCallArgs<C>): Promise<string | null> {
    try {
      // TypeScript can't relate a generic call's arguments to its name, hence the cast.
      const call = this.conn.call.bind(this.conn) as (name: C, args: unknown) => Promise<Reply<C>>;
      const r = await call(name, { ...args, game: this.gameId });
      return failed(r) ? errorText(r.err) : null;
    } catch (e) {
      return String(e);
    }
  }

  connectionStatus(): ConnectionStatus {
    return this.conn.status;
  }

  onConnectionStatus(handler: (s: ConnectionStatus) => void): () => void {
    return this.conn.onStatus(handler);
  }

  dispose(): void {
    window.clearInterval(this.heartbeat);
    this.rejoinListeners.clear();
    this.unsubscribe.forEach((u) => u());
  }
}
