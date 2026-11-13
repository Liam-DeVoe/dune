// The WebSocket connection to the Rust server (rust/docs/protocol.md). One per browser tab.
//
// - call(method, ...args) sends {"type":"call",...} and resolves with the reply's Result. A reply
//   with "error" (unknown method, bad arguments, a server exception) rejects.
// - on(name, handler) receives notifications ({"type":"notify",...}): GameView, JoinGame, ...
// - When the socket closes, every pending call rejects and the connection reconnects after the
//   delays of the C# client's RetryPolicy, and gives up after about an hour (`status`, for the
//   board's Disconnection banner). onOpen handlers run again, so a game screen can call
//   RequestReconnectGame anew, as the C# client does after SignalR reconnects.
// - The server knows who the user is per socket (protocol.md, Connection). A connection with a
//   session (`setSession`, the token of a login) logs every new socket in with
//   RequestResumeSession before anything else goes out. When the server says the session ended,
//   onSessionEnded handlers run. Session tokens never go in URLs: the page keeps them in
//   sessionStorage, and in localStorage when the user wants to stay logged in (`storeSession`).

import { withBase } from "../base";

/** `Result<T>` (Shared/Model/Result.cs), as the server sends it. */
export interface CallResult<T = unknown> {
  Success: boolean;
  Error: number;
  ErrorDetails: string;
  Contents: T | null;
}

/** `LoginInfo` (protocol.md, Payload objects). `Token` is set only by the calls that start a
 * session. */
export interface LoginInfo {
  UserId: number;
  Token: string | null;
  PlayerName: string;
  UserName: string;
  Email: string;
}

/** `GameInitInfo` without the savegame (protocol.md, Payload objects). */
export interface GameInitInfo {
  GameId: string;
  GameName: string | null;
  Participation: unknown;
  PhaseControl: unknown;
}

/** `ErrorType` values the client checks. `UserTokenNotFound`: the connection isn't logged in,
 * or its session ended. */
export const ErrorType = { None: 0, UserNameExists: 7, UserTokenNotFound: 8, InvalidGameEvent: 23 } as const;

const SESSION_KEY = "treachery.online;session";

/** The session token this tab logged in with, or the one kept from an earlier visit. */
export function storedSession(): string | null {
  return sessionStorage.getItem(SESSION_KEY) || localStorage.getItem(SESSION_KEY) || null;
}

/** Keeps a session token for this tab, and with `remember` for later visits too. */
export function storeSession(token: string, remember: boolean): void {
  sessionStorage.setItem(SESSION_KEY, token);
  if (remember) localStorage.setItem(SESSION_KEY, token);
  else localStorage.removeItem(SESSION_KEY);
}

/** Whether the user asked to stay logged in on this browser. */
export function sessionIsRemembered(): boolean {
  return localStorage.getItem(SESSION_KEY) != null;
}

export function clearStoredSession(): void {
  sessionStorage.removeItem(SESSION_KEY);
  localStorage.removeItem(SESSION_KEY);
}

/** `Game.LatestVersion`, which `RequestLogin` must name. */
export const LATEST_VERSION = 188;

/** `GameListScope.None`, for heartbeats that need no game list. */
export const GameListScopeNone = 1;

/** The C# client's `Support.GetHash`: SHA-256 of the UTF-8 bytes as 64 lowercase hex digits. */
export async function hashPassword(password: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(password));
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

type Handler = (...args: unknown[]) => void;

/** `Client.IsConnected` and why not: "connecting" before the first open, "reconnecting" while
 * the retry policy runs after the socket closed, "lost" once it gave up. */
export type ConnectionStatus = "connecting" | "connected" | "reconnecting" | "lost";

/** RetryPolicy.cs: the delay before reconnection attempt `n` (from 0), in ms, or null to give
 * up (about an hour after the disconnection). */
export function retryDelay(n: number): number | null {
  if (n === 0) return 0;
  if (n < 5) return 1000;
  if (n < 30) return 2000;
  if (n < 100) return 10000;
  if (n < 200) return 30000;
  return null;
}

interface Pending {
  resolve: (r: CallResult) => void;
  reject: (e: Error) => void;
}

export function serverUrl(): string {
  return `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}${withBase("ws")}`;
}

export class Connection {
  private socket: WebSocket | null = null;
  private nextId = 1;
  private readonly pending = new Map<number, Pending>();
  private readonly handlers = new Map<string, Set<Handler>>();
  private readonly openHandlers = new Set<() => void>();
  private opened: Promise<void> | null = null;
  private closed = false;
  private retries = 0;
  private retryTimer: number | undefined;
  private currentStatus: ConnectionStatus = "connecting";
  private readonly statusHandlers = new Set<(s: ConnectionStatus) => void>();
  private session: string | null = null;
  private readonly sessionEndedHandlers = new Set<() => void>();

  constructor(private readonly url: string = serverUrl()) {
    this.on("SessionEnded", () => this.sessionEnded());
  }

  /** The session every socket of this connection logs in with; null for none. Setting one
   * doesn't log the open socket in: the call that started the session did, or call
   * `resumeSession`. */
  setSession(token: string | null): void {
    this.session = token;
  }

  get hasSession(): boolean {
    return this.session != null;
  }

  /** Logs the open socket in with `token` and keeps it for later sockets. Resolves with the
   * user's details, or null when the session is unknown or over. */
  async resumeSession(token: string): Promise<LoginInfo | null> {
    this.session = token;
    const r = await this.call<LoginInfo>("RequestResumeSession", token);
    if (r.Success) return r.Contents;
    this.session = null;
    return null;
  }

  /** Runs when the server ends this connection's session (logout elsewhere, a new password,
   * the account deleted) or refuses to resume it. Returns an unsubscribe function. */
  onSessionEnded(handler: () => void): () => void {
    this.sessionEndedHandlers.add(handler);
    return () => this.sessionEndedHandlers.delete(handler);
  }

  private sessionEnded(): void {
    this.session = null;
    this.sessionEndedHandlers.forEach((h) => h());
  }

  /** Opens the socket (once) and resolves when it is open. */
  open(): Promise<void> {
    if (this.opened) return this.opened;
    this.opened = new Promise((resolve, reject) => {
      const socket = new WebSocket(this.url);
      this.socket = socket;
      socket.onopen = () => {
        this.retries = 0;
        const ready = () => {
          this.setStatus("connected");
          resolve();
          this.openHandlers.forEach((h) => h());
        };
        // A new socket is nobody until it logs in: resume the session first.
        const token = this.session;
        if (token == null) return ready();
        this.send<LoginInfo>("RequestResumeSession", [token])
          .then((r) => { if (!r.Success && this.session === token) this.sessionEnded(); })
          .catch(() => {})
          .finally(ready);
      };
      socket.onmessage = (e) => this.receive(String(e.data));
      socket.onerror = () => reject(new Error(`Cannot connect to ${this.url}`));
      socket.onclose = () => {
        this.socket = null;
        this.opened = null;
        for (const p of this.pending.values()) p.reject(new Error("The connection to the server closed."));
        this.pending.clear();
        if (this.closed) return;
        // A first connection that never opened keeps "connecting"; the retry policy applies
        // once a connection was made, as SignalR's automatic reconnect does.
        if (this.currentStatus === "connecting") {
          this.retryTimer = window.setTimeout(() => this.retry(), 2000);
          return;
        }
        const delay = retryDelay(this.retries++);
        if (delay === null) {
          this.setStatus("lost");
          return;
        }
        this.setStatus("reconnecting");
        this.retryTimer = window.setTimeout(() => this.retry(), delay);
      };
    });
    return this.opened;
  }

  private retry(): void {
    this.retryTimer = undefined;
    if (!this.closed && !this.opened) this.open().catch(() => {});
  }

  /** Whether the socket is open, and if not, whether it is still trying. */
  get status(): ConnectionStatus {
    return this.currentStatus;
  }

  /** Runs when `status` changes. Returns an unsubscribe function. */
  onStatus(handler: (s: ConnectionStatus) => void): () => void {
    this.statusHandlers.add(handler);
    return () => this.statusHandlers.delete(handler);
  }

  private setStatus(s: ConnectionStatus): void {
    if (s === this.currentStatus) return;
    this.currentStatus = s;
    this.statusHandlers.forEach((h) => h(s));
  }

  /** Runs after every (re)connection. */
  onOpen(handler: () => void): () => void {
    this.openHandlers.add(handler);
    return () => this.openHandlers.delete(handler);
  }

  close(): void {
    this.closed = true;
    window.clearTimeout(this.retryTimer);
    this.socket?.close();
  }

  async call<T = unknown>(method: string, ...args: unknown[]): Promise<CallResult<T>> {
    await this.open();
    return this.send<T>(method, args);
  }

  /** Sends a call on the socket, open or not yet resolved as open. */
  private send<T>(method: string, args: unknown[]): Promise<CallResult<T>> {
    const id = this.nextId++;
    return new Promise<CallResult<T>>((resolve, reject) => {
      if (!this.socket) return reject(new Error("The connection to the server closed."));
      this.pending.set(id, { resolve: resolve as (r: CallResult) => void, reject });
      this.socket.send(JSON.stringify({ type: "call", id, method, args }));
    });
  }

  /** A call whose failure is an exception: resolves with `Contents`. */
  async invoke<T = unknown>(method: string, ...args: unknown[]): Promise<T> {
    const r = await this.call<T>(method, ...args);
    if (!r.Success) throw new Error(`${method}: error ${r.Error}${r.ErrorDetails ? `: ${r.ErrorDetails}` : ""}`);
    return r.Contents as T;
  }

  on(name: string, handler: Handler): () => void {
    let set = this.handlers.get(name);
    if (!set) this.handlers.set(name, (set = new Set()));
    set.add(handler);
    return () => set!.delete(handler);
  }

  private receive(text: string): void {
    const m = JSON.parse(text) as { type: string; id?: number; result?: CallResult; error?: string; name?: string; args?: unknown[] };
    if (m.type === "reply" && m.id != null) {
      const p = this.pending.get(m.id);
      if (!p) return;
      this.pending.delete(m.id);
      if (m.error != null) p.reject(new Error(m.error));
      else p.resolve(m.result!);
    } else if (m.type === "notify" && m.name) {
      this.handlers.get(m.name)?.forEach((h) => h(...(m.args ?? [])));
    }
  }
}

/** Logs the connection in, creating the account first when `create` is set and the name is
 * free. The connection keeps the new session for its later sockets. */
export async function login(conn: Connection, userName: string, password: string, create?: { email: string | null; playerName: string }): Promise<LoginInfo> {
  const hashed = await hashPassword(password);
  await conn.invoke("Connect");
  let info: LoginInfo | null = null;
  if (create) {
    const r = await conn.call<LoginInfo>("RequestCreateUser", userName, hashed, create.email, create.playerName);
    if (r.Success) info = r.Contents!;
    else if (r.Error !== ErrorType.UserNameExists) throw new Error(`RequestCreateUser: error ${r.Error}`);
  }
  info ??= await conn.invoke<LoginInfo>("RequestLogin", LATEST_VERSION, userName, hashed);
  conn.setSession(info.Token);
  return info;
}
