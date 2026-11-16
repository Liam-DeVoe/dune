// The WebSocket connection to the Rust server (rust/docs/protocol.md). One per browser tab.
//
// - call(name, args) sends {"id","call","args"} and resolves with the reply: {ok} with the call's
//   reply type, or {err: {code, message}}. The types come from rust/protocol
//   (web/src/protocol), so a call's arguments and reply are checked here.
// - on(name, handler) receives notifications ({"notify","data"}): gameView, joinGame, ...
// - When the socket closes, every pending call rejects and the connection reconnects after the
//   delays of the C# client's RetryPolicy, and gives up after about an hour (`status`, for the
//   board's Disconnection banner). onOpen handlers run again, so a game screen can call
//   reconnectGame anew, as the C# client does after SignalR reconnects.
// - The server knows who the user is per socket (protocol.md, Connection). A connection with a
//   session (`setSession`, the token of a login) authenticates every new socket with it before
//   anything else goes out. When the server says the session ended, onSessionEnded handlers
//   run. Session tokens never go in URLs: the page keeps them in sessionStorage, and in
//   localStorage when the user wants to stay logged in (`storeSession`).

import { withBase } from "../base";
import { LATEST_VERSION } from "../generated/version";
import type { Account } from "../protocol/Account";
import type { Call } from "../protocol/Call";
import type { CallError } from "../protocol/CallError";
import type { CallReplies } from "../protocol/CallReplies";
import type { ErrorCode } from "../protocol/ErrorCode";
import type { NewSession } from "../protocol/NewSession";
import type { Notification } from "../protocol/Notification";

export type { CallError, ErrorCode };

/** A call's name. */
export type CallName = Call["call"];

/** The arguments of a call, as a parameter list: empty for a call without arguments. */
export type CallArgs<C extends CallName> = Extract<Call, { call: C }> extends { args: infer A } ? [args: A] : [];

/** A call's reply: `ok` with the call's reply type, or `err`. */
export type Reply<C extends CallName> = { ok: CallReplies[C]; err?: undefined } | { ok?: undefined; err: CallError };

/** A notification's name. */
export type NotificationName = Notification["notify"];

/** A notification's data. */
export type NotificationData<N extends NotificationName> = Extract<Notification, { notify: N }> extends { data: infer D } ? D : undefined;

/** Whether a reply is a failure. */
export function failed<C extends CallName>(r: Reply<C>): r is { err: CallError } {
  return r.err !== undefined;
}

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

/** The C# client's `Support.GetHash`: SHA-256 of the UTF-8 bytes as 64 lowercase hex digits. */
export async function hashPassword(password: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(password));
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

type Handler = (data: never) => void;

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
  resolve: (r: { ok?: unknown; err?: CallError }) => void;
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
    this.on("sessionEnded", () => this.sessionEnded());
  }

  /** The session every socket of this connection authenticates with; null for none. Setting one
   * doesn't authenticate the open socket: the call that started the session did, or call
   * `authenticate`. */
  setSession(token: string | null): void {
    this.session = token;
  }

  get hasSession(): boolean {
    return this.session != null;
  }

  /** Authenticates the open socket with `token` and keeps it for later sockets. Resolves with
   * the user's account, or null when the session is unknown or over. */
  async authenticate(token: string): Promise<Account | null> {
    this.session = token;
    const r = await this.call("authenticate", { token });
    if (!failed(r)) return r.ok;
    this.session = null;
    return null;
  }

  /** Runs when the server ends this connection's session (logout elsewhere, a new password,
   * the account deleted) or refuses to authenticate with it. Returns an unsubscribe function. */
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
        // A new socket is nobody until it authenticates: do that first.
        const token = this.session;
        if (token == null) return ready();
        this.send<"authenticate">({ call: "authenticate", args: { token } })
          .then((r) => { if (failed(r) && this.session === token) this.sessionEnded(); })
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

  /** Makes a call once the socket is open. A refusal resolves with `err`; a closed socket
   * rejects. */
  async call<C extends CallName>(name: C, ...args: CallArgs<C>): Promise<Reply<C>> {
    await this.open();
    return this.send<C>(args.length > 0 ? { call: name, args: args[0] } : { call: name });
  }

  /** Sends a call on the socket, open or not yet resolved as open. */
  private send<C extends CallName>(call: { call: C; args?: unknown }): Promise<Reply<C>> {
    const id = this.nextId++;
    return new Promise<Reply<C>>((resolve, reject) => {
      if (!this.socket) return reject(new Error("The connection to the server closed."));
      this.pending.set(id, { resolve: resolve as Pending["resolve"], reject });
      this.socket.send(JSON.stringify({ id, ...call }));
    });
  }

  /** A call whose failure is an exception: resolves with `ok`. */
  async invoke<C extends CallName>(name: C, ...args: CallArgs<C>): Promise<CallReplies[C]> {
    const r = await this.call(name, ...args);
    if (failed(r)) throw new Error(`${name}: ${r.err.message}`);
    return r.ok;
  }

  on<N extends NotificationName>(name: N, handler: (data: NotificationData<N>) => void): () => void {
    let set = this.handlers.get(name);
    if (!set) this.handlers.set(name, (set = new Set()));
    set.add(handler as Handler);
    return () => set!.delete(handler as Handler);
  }

  private receive(text: string): void {
    const m = JSON.parse(text) as { id?: number; ok?: unknown; err?: CallError } | Notification;
    if ("notify" in m) {
      const data = "data" in m ? m.data : undefined;
      this.handlers.get(m.notify)?.forEach((h) => (h as (d: unknown) => void)(data));
    } else if (m.id != null) {
      const p = this.pending.get(m.id);
      if (!p) return;
      this.pending.delete(m.id);
      p.resolve(m);
    }
  }
}

/** Logs the connection in, creating the account first when `create` is set and the name is
 * free. The connection keeps the new session for its later sockets. */
export async function login(conn: Connection, userName: string, password: string, create?: { email: string | null; playerName: string }): Promise<NewSession> {
  const passwordHash = await hashPassword(password);
  await conn.invoke("serverInfo");
  let session: NewSession | null = null;
  if (create) {
    const r = await conn.call("createUser", { userName, passwordHash, email: create.email, playerName: create.playerName });
    if (!failed(r)) session = r.ok;
    else if (r.err.code !== "userNameExists") throw new Error(`createUser: ${r.err.message}`);
  }
  session ??= await conn.invoke("login", { version: LATEST_VERSION, userName, passwordHash });
  conn.setSession(session.token);
  return session;
}
