// Where views come from. The game screen renders whatever the current source holds and never
// computes game rules itself (rust/docs/client.md, State).
//
// - StaticSource: a view saved as JSON (web/public/views), for the skeleton and component tests.
// - WebSocketSource: a game on the Rust server. It gets a view after every event and every
//   participation change, and answers draft queries and submissions over the protocol in
//   rust/docs/protocol.md.
//
// Events, in drafts and submissions, are in the savegame format: PascalCase members, enums as
// numbers (FactionValue and friends in generated/enums.ts). The source fills in `Initiator`.
import type { DraftResult } from "../generated/DraftResult";
import type { GameView } from "../generated/GameView";
import type { ReportLines } from "../generated/ReportLines";
import type { Part } from "../generated/Part";
import { FactionValue } from "../generated/enums";
import { type CallResult, type Connection, type ConnectionStatus, GameListScopeNone } from "./connection";

export type GameEventDraft = Record<string, unknown> & { $type: string };

/** One entry of `RequestUndoList`. */
export interface UndoEntry {
  eventNumber: number;
  text: Part[];
  /** `UndoSession.IsListed`. */
  listed: boolean;
  /** The turn and main phase the event happened in (`UndoSession.MomentOf`). */
  turn: number;
  phase: string;
}

export interface ViewSource {
  load(): Promise<GameView>;
  /** Later views, for sources whose game moves on. Returns an unsubscribe function. */
  subscribe?(onView: (view: GameView) => void): () => void;
  /** A draft query: the form's inputs as an event in savegame JSON. */
  query(draft: GameEventDraft): Promise<DraftResult>;
  /** Submits an event. Resolves with the server's error text, or null when it was accepted. */
  submit(event: GameEventDraft): Promise<string | null>;
  /** `RequestReport`: the lines of one report of `log.reports`, for the log's history. */
  report?(index: number): Promise<ReportLines>;
  /** `RequestUndoList`, for hosts (the undo panel). */
  undoList?(): Promise<UndoEntry[]>;
  /** `RequestUndoPreview`, for hosts: the view at gap `gap` (just before event `gap`), without
   * actions, for the undo timeline's preview. */
  undoPreview?(gap: number): Promise<GameView>;
  /** `RequestSaveGame`, for hosts and the admin: the live game as a savegame. */
  saveGame?(): Promise<string>;
  /** A game method whose first arguments are userToken and gameId (`RequestUndo`,
   * `RequestSetBotSpeed`, `SendChatMessage`, ...). Resolves with the server's error text, or
   * null when it succeeded. */
  gameCall?(method: string, ...args: unknown[]): Promise<string | null>;
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

function errorText(r: CallResult): string {
  return r.ErrorDetails || `error ${r.Error}`;
}

/** Heartbeats keep the user in the server's user list and run its housekeeping (protocol.md). */
const HEARTBEAT_MS = 5000;

export class WebSocketSource implements ViewSource {
  private view: GameView | null = null;
  private readonly listeners = new Set<(view: GameView) => void>();
  private first: Promise<GameView>;
  private resolveFirst!: (view: GameView) => void;
  private readonly unsubscribe: (() => void)[] = [];
  private heartbeat: number | undefined;

  constructor(private readonly conn: Connection, readonly userToken: string, readonly gameId: string) {
    this.first = new Promise((resolve) => { this.resolveFirst = resolve; });
    this.unsubscribe.push(conn.on("GameView", (v) => this.receive(v as GameView)));
    // After a reconnection the socket is a new connection outside the game's group: rejoin it.
    this.unsubscribe.push(conn.onOpen(() => { if (this.view) this.join().catch(() => {}); }));
  }

  private receive(view: GameView): void {
    this.view = view;
    this.resolveFirst(view);
    this.listeners.forEach((l) => l(view));
  }

  private async join(): Promise<void> {
    await this.conn.invoke("Connect");
    // GetLoginInfo restores the server's user cache after a restart, as the C# client's
    // reconnect does; RequestReconnectGame adds this connection to the game and sends the view.
    await this.conn.call("GetLoginInfo", this.userToken);
    const r = await this.conn.call("RequestReconnectGame", this.userToken, this.gameId);
    if (!r.Success) throw new Error(`RequestReconnectGame: ${errorText(r)}`);
  }

  /** Joins the game's group and resolves with the first view. */
  async load(): Promise<GameView> {
    if (this.view) return this.view;
    await this.conn.open();
    await this.join();
    if (this.heartbeat === undefined) {
      this.heartbeat = window.setInterval(() => {
        this.conn.call("RequestHeartbeat", this.userToken, GameListScopeNone).catch(() => {});
      }, HEARTBEAT_MS);
    }
    return this.first;
  }

  subscribe(onView: (view: GameView) => void): () => void {
    this.listeners.add(onView);
    return () => this.listeners.delete(onView);
  }

  private withInitiator(event: GameEventDraft): GameEventDraft {
    const faction = this.view?.viewer.faction;
    const { $type, ...rest } = event;
    // `$type` must come first (protocol.md, Encoding).
    return { $type, ...rest, Initiator: faction ? FactionValue[faction] : 0 };
  }

  async query(draft: GameEventDraft): Promise<DraftResult> {
    const r = await this.conn.call<DraftResult>("QueryDraft", this.userToken, this.gameId, this.view?.eventNumber ?? 0, this.withInitiator(draft));
    if (r.Success) return r.Contents!;
    // A refused query leaves the form's options and shows why, as a validation message.
    const action = this.view?.actions.find((a) => a.eventType === draft.$type);
    return { eventType: draft.$type, options: action?.options ?? null, validation: [{ t: "text", v: errorText(r) }] };
  }

  async submit(event: GameEventDraft): Promise<string | null> {
    try {
      const r = await this.conn.call("SubmitEvent", this.userToken, this.gameId, this.withInitiator(event));
      return r.Success ? null : errorText(r);
    } catch (e) {
      return String(e);
    }
  }

  /** `RequestReport`: the lines of an earlier report. */
  report(index: number): Promise<ReportLines> {
    return this.conn.invoke<ReportLines>("RequestReport", this.userToken, this.gameId, index);
  }

  /** `RequestUndoList`, for hosts. */
  undoList(): Promise<UndoEntry[]> {
    return this.conn.invoke<UndoEntry[]>("RequestUndoList", this.userToken, this.gameId);
  }

  /** `RequestUndoPreview`, for hosts. */
  undoPreview(gap: number): Promise<GameView> {
    return this.conn.invoke<GameView>("RequestUndoPreview", this.userToken, this.gameId, gap);
  }

  /** `RequestSaveGame`, for hosts and the admin. */
  saveGame(): Promise<string> {
    return this.conn.invoke<string>("RequestSaveGame", this.userToken, this.gameId);
  }

  async gameCall(method: string, ...args: unknown[]): Promise<string | null> {
    try {
      const r = await this.conn.call(method, this.userToken, this.gameId, ...args);
      return r.Success ? null : errorText(r);
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
    this.unsubscribe.forEach((u) => u());
  }
}
