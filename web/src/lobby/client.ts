// The lobby half of the client: the connection, the
// logged-in user, the server status (game lists), and the game the user is in, with its
// participation, settings and latest view. The game screen itself renders views from a
// WebSocketSource (state/source.ts); this store only says which screen of Main.razor to show and
// holds what those screens read beside the view.
//
// Every request* method returns what the C# method returns: an error text for the screen, or
// empty/null on success. The texts are the server's (`err.message`).
import {
  type CallArgs, type CallName, clearStoredSession, Connection, failed, type Reply, sessionIsRemembered, storedSession, storeSession,
} from "../state/connection";
import { WebSocketSource, type GameEventDraft } from "../state/source";
import type { GameView } from "../generated/GameView";
import { PhaseValue, type Faction, type Phase, type Ruleset } from "../generated/enums";
import type { NewSession } from "../protocol/NewSession";
import { loadSkin, type Skin } from "../skin";
import { withBase } from "../base";
import { LATEST_VERSION } from "../generated/version";
import { loadLobby, type Lobby } from "./data";
import { Participation } from "./participation";
import {
  type Account, type AdminInfo, canBeJoined, defaultSettings, type GameInfo, type GameInitInfo, type GameListScope, type GameSettings, type IntDict, settingsOf,
  type LobbyStatus, type PhaseControlInfo, type ScheduledGameInfo, type SeenUser, type ServerInfo, type SubscriptionType, type UserStatus,
} from "./types";

/** `IGameService.HeartbeatDelay`. */
export const HEARTBEAT_DELAY = 6000;

/** How the page was opened: the lobby, a game to rejoin (`/game/:gameId`), or the login link
 * of an async-play mail (`/link/:linkToken`). */
export interface StartOptions {
  gameId?: string;
  linkToken?: string;
}

/** The game the user is in (`Client.Game`, `GameId`, `GameName`, ...). */
export interface CurrentGame {
  gameId: string;
  gameName: string | null;
  participation: Participation;
  /** `Game.Settings`: `new GameSettings()` until the host's `UpdateSettings` arrives. */
  settings: GameSettings;
  phaseControl: PhaseControlInfo | null;
  /** The latest view; null until the first one arrives. */
  view: GameView | null;
  source: WebSocketSource;
}

export interface LobbyState {
  connected: boolean;
  serverInfo: ServerInfo | null;
  /** The user's account; null until they log in. */
  account: Account | null;
  ownGames: GameInfo[];
  activeGames: GameInfo[];
  activeGamesWithOpenSeats: GameInfo[];
  activeGamesWithoutOpenSeats: GameInfo[];
  scheduledGames: ScheduledGameInfo[];
  recentlySeenUsers: Map<number, SeenUser>;
  game: CurrentGame | null;
  reseatRequested: boolean;
  adminInfo: AdminInfo | null;
  /** Bumped by `Refresh`, for components that redraw on a timer. */
  version: number;
}

function phaseOf(view: GameView | null): Phase {
  return (view?.phase ?? "AwaitingPlayers") as Phase;
}

export class LobbyClient {
  private state: LobbyState = {
    connected: false, serverInfo: null, account: null, ownGames: [], activeGames: [], activeGamesWithOpenSeats: [],
    activeGamesWithoutOpenSeats: [], scheduledGames: [], recentlySeenUsers: new Map(), game: null, reseatRequested: false,
    adminInfo: null, version: 0,
  };
  private readonly listeners = new Set<() => void>();
  /** Whether the user wants to stay logged in on this browser. */
  private remember = sessionIsRemembered();
  private heartbeatTimer: number | undefined;
  private viewUnsubscribe: (() => void) | undefined;
  readonly conn: Connection;
  /** `Client.IsAdmin`: set by the admin page, which asks for every game. */
  isAdmin = false;

  constructor(readonly lobby: Lobby, readonly skin: Skin, conn?: Connection) {
    this.conn = conn ?? new Connection();
    this.conn.on("joinGame", ({ user, name, seat }) => this.inGame((g) => g.participation.addPlayer(user, name, seat)));
    this.conn.on("observeGame", ({ user, name }) => this.inGame((g) => g.participation.addObserver(user, name)));
    this.conn.on("setOrUnsetHost", ({ user }) => this.inGame((g) => g.participation.setOrUnsetHost(user)));
    this.conn.on("openOrCloseSeats", ({ seats }) => this.inGame((g) => seats.forEach((s) => g.participation.openOrCloseSeat(s))));
    this.conn.on("assignSeats", ({ seats }) => this.inGame((g) => g.participation.assignSeats(seats)));
    this.conn.on("botSpeed", ({ speed }) => this.inGame((g) => { g.participation.botsSpeed = speed; }));
    this.conn.on("updateSettings", ({ settings }) => this.inGame((g) => { g.settings = settingsOf(settings); }));
    this.conn.on("phaseControl", (info) => this.inGame((g) => { g.phaseControl = info; }));
    this.conn.on("removeUser", ({ user, kick }) => { void this.handleRemoveUser(user, kick); });
    this.conn.on("loadGame", (info) => { void this.loadGame(info); });
    this.conn.onOpen(() => { this.set({ connected: true }); });
    this.conn.onSessionEnded(() => { void this.sessionEnded(); });
    // Where earlier versions kept the token of a session that no longer exists.
    localStorage.removeItem("treachery.online;auth.token");
  }

  // The external store (useSyncExternalStore).

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getState = (): LobbyState => this.state;

  private set(patch: Partial<LobbyState>): void {
    this.state = { ...this.state, ...patch, version: this.state.version + 1 };
    this.listeners.forEach((l) => l());
  }

  /** `Client.Refresh`: redraw. */
  refresh(): void {
    this.set({});
  }

  private inGame(change: (g: CurrentGame) => void): void {
    const g = this.state.game;
    if (!g) return;
    const next = { ...g, participation: g.participation.clone() };
    change(next);
    this.set({ game: next });
  }

  // Derived members of Client.cs.

  get loggedIn(): boolean { return this.state.account != null; }
  get userId(): number { return this.state.account?.userId ?? -1; }
  get inGameNow(): boolean { return this.state.game != null; }
  get isHost(): boolean { return this.state.game?.participation.isHost(this.userId) ?? false; }
  get isObserver(): boolean { return this.state.game?.participation.isObserver(this.userId) ?? false; }

  /** `Client.UserStatus`: null until the lobby lists the user. */
  userStatus(): UserStatus | null {
    return this.state.recentlySeenUsers.get(this.userId)?.status ?? null;
  }

  /** `Client.PlayersNeedSeating`. */
  playersNeedSeating(): boolean {
    const g = this.state.game;
    if (!g) return false;
    return this.state.reseatRequested
      || (PhaseValue[phaseOf(g.view)] >= PhaseValue.TradingFactions && g.participation.seatedPlayers.values().includes(-1));
  }

  /** `Client.InScheduledMaintenance`. */
  inScheduledMaintenance(): boolean {
    const info = this.state.serverInfo;
    if (!info) return false;
    const start = Date.parse(info.scheduledMaintenance);
    const now = Date.now();
    const hoursUntil = (start - now) / 3_600_000;
    const g = this.state.game;
    return start + 15 * 60_000 > now
      && ((g != null && hoursUntil < 6 && PhaseValue[phaseOf(g.view)] <= PhaseValue.AwaitingPlayers) || hoursUntil < 1);
  }

  // Start (Client.Start).

  async start({ gameId, linkToken }: StartOptions = {}): Promise<void> {
    await this.conn.open();
    this.set({ connected: true });
    await this.connect();
    if (linkToken) {
      // A link works once: take it out of the address bar, then trade it for a session.
      window.history.replaceState(null, "", withBase(""));
      const r = await this.conn.call("linkLogin", { token: linkToken });
      if (!failed(r)) {
        this.loggedInWith(r.ok.session);
        gameId = r.ok.gameId ?? undefined;
        if (gameId) window.history.replaceState(null, "", withBase(`game/${gameId}`));
      }
    }
    if (!this.loggedIn) await this.tryResumeSavedLogin();
    if (this.loggedIn && gameId) await this.enterGame(gameId, null);
    else if (this.loggedIn) await this.sendHeartbeatAndGetServerStatus();
    this.scheduleHeartbeat();
  }

  private async connect(): Promise<void> {
    const r = await this.conn.call("serverInfo");
    if (!failed(r)) this.set({ serverInfo: r.ok });
  }

  private scheduleHeartbeat(): void {
    window.clearTimeout(this.heartbeatTimer);
    this.heartbeatTimer = window.setTimeout(async () => {
      try {
        if (this.state.connected && this.loggedIn) await this.sendHeartbeatAndGetServerStatus();
      } catch (e) {
        console.log(String(e));
      }
      this.scheduleHeartbeat();
    }, HEARTBEAT_DELAY);
  }

  /** `Client.Invoke`. A call refused because the connection's session is over logs the user out. */
  private async invoke<C extends CallName>(name: C, ...args: CallArgs<C>): Promise<Reply<C>> {
    const r = await this.conn.call(name, ...args);
    if (failed(r) && r.err.code === "notLoggedIn" && this.loggedIn) await this.sessionEnded();
    return r;
  }

  /** The error text of a reply, empty for a success. */
  private describe<C extends CallName>(r: Reply<C>): string {
    return failed(r) ? r.err.message : "";
  }

  // Authentication.

  /** A call that started a session authenticated the connection: keep its token for this tab
   * (and later visits, if the user wants), so a reload or a new socket authenticates again. */
  private loggedInWith(session: NewSession): void {
    this.conn.setSession(session.token);
    storeSession(session.token, this.remember);
    this.set({ account: session.account });
  }

  /** Resolves with the error text, or null once logged in. */
  async requestCreateUser(userName: string, passwordHash: string, email: string | null, playerName: string): Promise<string | null> {
    const r = await this.conn.call("createUser", { userName, passwordHash, email, playerName });
    if (failed(r)) return r.err.message;
    this.loggedInWith(r.ok);
    return null;
  }

  /** Resolves with the error text, or null once logged in. */
  async requestLogin(userName: string, passwordHash: string): Promise<string | null> {
    const r = await this.conn.call("login", { version: LATEST_VERSION, userName, passwordHash });
    if (failed(r)) return r.err.message;
    this.loggedInWith(r.ok);
    await this.sendHeartbeatAndGetServerStatus();
    return null;
  }

  /** Ends the session on the server, for every tab that uses it. */
  async logout(): Promise<void> {
    if (this.conn.hasSession) await this.conn.call("logout").catch(() => {});
    await this.sessionEnded();
  }

  /** The session is over: forget it and show the login. */
  private async sessionEnded(): Promise<void> {
    this.conn.setSession(null);
    clearStoredSession();
    if (this.state.game) {
      this.leaveSource();
      this.set({ game: null, reseatRequested: false });
    }
    this.set({ account: null });
  }

  /** Whether a reload or a later visit logs in again without credentials. */
  rememberLogin(keep: boolean): void {
    this.remember = keep;
    const token = storedSession();
    if (token) storeSession(token, keep);
  }

  private async tryResumeSavedLogin(): Promise<boolean> {
    const saved = storedSession();
    if (!saved) return false;
    const account = await this.conn.authenticate(saved);
    if (!account) {
      clearStoredSession();
      return false;
    }
    this.set({ account });
    return true;
  }

  /** Resolves with the error text, or null once the token is mailed. */
  async requestPasswordReset(userNameOrEmail: string): Promise<string | null> {
    const r = await this.conn.call("requestPasswordReset", { userNameOrEmail });
    return failed(r) ? r.err.message : null;
  }

  /** Resolves with the error text, or null once logged in. */
  async requestSetPassword(userName: string, resetToken: string, passwordHash: string): Promise<string | null> {
    const r = await this.conn.call("setPassword", { userName, resetToken, passwordHash });
    if (failed(r)) return r.err.message;
    this.loggedInWith(r.ok);
    await this.sendHeartbeatAndGetServerStatus();
    return null;
  }

  /** An empty `passwordHash` keeps the password. */
  async requestUpdateUserInfo(passwordHash: string, email: string, playerName: string): Promise<string | null> {
    if (!this.loggedIn) return "Not logged in";
    const r = await this.invoke("updateUserInfo", { passwordHash, playerName, email });
    if (failed(r)) return r.err.message;
    this.set({ account: r.ok });
    return null;
  }

  async requestSetUserStatus(status: UserStatus): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    return this.updateServerStatusOnSuccess(await this.invoke("setUserStatus", { status }));
  }

  // Game management.

  async requestCreateGame(name: string, password: string, savegame: string | null = null, skin: string | null = null): Promise<string | null> {
    if (!this.loggedIn) return "Not logged in";
    const r = await this.invoke("createGame", { name, passwordHash: password, savegame, skin });
    if (failed(r)) return r.err.message;
    return this.loadGame(r.ok);
  }

  async requestScheduleGame(starts: string, ruleset: Ruleset | null, players: number, maxTurns: number, factions: Faction[], asyncPlay: boolean): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke("scheduleGame", { starts, ruleset, players, maxTurns, factions, asyncPlay }));
  }

  async requestCancelGame(scheduledGame: string): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke("cancelScheduledGame", { scheduledGame }));
  }

  async requestCloseGame(game: string): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke("closeGame", { game }));
  }

  private updateServerStatusOnSuccess(r: Reply<"heartbeat">): string {
    if (failed(r)) return r.err.message;
    this.handleUpdatedServerStatus(r.ok);
    return "";
  }

  async requestUpdateSettings(game: string, settings: GameSettings): Promise<string> {
    return this.describe(await this.invoke("updateSettings", { game, settings }));
  }

  /** `seat` -1 joins without a particular seat. */
  async requestJoinGame(game: string, password: string, seat: number): Promise<string | null> {
    const r = await this.invoke("joinGame", { game, passwordHash: password, seat: seat < 0 ? null : seat });
    if (failed(r)) return r.err.message;
    return this.loadGame(r.ok);
  }

  async requestSubscribeGame(scheduledGame: string, subscription: SubscriptionType): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke("subscribe", { scheduledGame, subscription }));
  }

  async requestObserveGame(game: string, password: string): Promise<string | null> {
    const r = await this.invoke("observeGame", { game, passwordHash: password });
    if (failed(r)) return r.err.message;
    return this.loadGame(r.ok);
  }

  /** A host's call about the current game: the error text, empty on success. */
  private async gameCall(call: (game: string) => Promise<Reply<CallName>>): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    const g = this.state.game;
    if (!g) return "Not in game";
    return this.describe(await call(g.gameId));
  }

  requestSetOrUnsetHost(user: number): Promise<string> { return this.gameCall((game) => this.invoke("setOrUnsetHost", { game, user })); }
  requestOpenOrCloseSeat(seat: number): Promise<string> { return this.gameCall((game) => this.invoke("openOrCloseSeat", { game, seat })); }
  requestKick(user: number): Promise<string> { return this.gameCall((game) => this.invoke("kick", { game, user })); }
  requestSetBotSpeed(speed: number): Promise<string> { return this.gameCall((game) => this.invoke("setBotSpeed", { game, speed })); }

  requestReseat(): void {
    this.set({ reseatRequested: true });
  }

  async requestLeaveGame(): Promise<string> {
    const g = this.state.game;
    if (!g) return "Not in game";
    return this.updateServerStatusOnSuccess(await this.invoke("leaveGame", { game: g.gameId }));
  }

  async requestAssignSeats(seats: IntDict<number>): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    if (!this.state.game) return "Not in game";
    this.set({ reseatRequested: false });
    return this.gameCall((game) => this.invoke("assignSeats", { game, seats }));
  }

  /**
   * `Client.RequestGameEvent`, for the setup screens' events: submits through the game's source,
   * then opens the empty seats when `EstablishPlayers` asked for it (`OpenEmptySeats`).
   */
  async requestGameEvent(event: GameEventDraft): Promise<string | null> {
    if (!this.loggedIn) return "Not logged in";
    const g = this.state.game;
    if (!g) return "Not in game";
    const result = await g.source.submit(event);
    if (event.type === "EstablishPlayers" && event.settings?.autoOpenEmptySeats) {
      const players = this.state.game?.view?.players ?? [];
      const seated = this.state.game?.participation.seatedPlayers.values() ?? [];
      for (const p of players.filter((p) => !seated.includes(p.seat))) await this.requestOpenOrCloseSeat(p.seat);
    }
    return result;
  }

  // Admin (Client.Admin*).

  /** An admin call's text: the server's answer, or the error. */
  private async adminCall<C extends CallName>(name: C, ...args: CallArgs<C>): Promise<string> {
    const r = await this.invoke(name, ...args);
    return failed(r) ? r.err.message : String(r.ok);
  }

  async adminUpdateMaintenance(starts: string): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    return this.adminCall("adminSetMaintenance", { starts });
  }

  async adminPersistState(): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    // As Client.AdminPersistState, which refuses outside a game (the admin page is never in one).
    if (!this.state.game) return "Not in game";
    return this.adminCall("adminSaveAll");
  }

  adminRestoreState(): Promise<string> { return this.adminCall("adminReloadAll"); }
  adminCloseGame(game: string): Promise<string> { return this.adminCall("adminCloseGame", { game }); }
  adminCancelGame(scheduledGame: string): Promise<string> { return this.adminCall("adminCancelScheduledGame", { scheduledGame }); }
  adminDeleteUser(user: number): Promise<string> { return this.adminCall("deleteUser", { user }); }

  async adminDownloadGame(gameId: string): Promise<string> {
    const r = await this.invoke("adminDownloadGame", { game: gameId });
    if (failed(r)) return r.err.message;
    const url = URL.createObjectURL(new Blob([r.ok], { type: "application/json" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `savegame-${gameId}.json`;
    a.click();
    URL.revokeObjectURL(url);
    return "Savegame downloaded";
  }

  async getAdminInfo(): Promise<string> {
    const r = await this.invoke("adminInfo");
    if (failed(r)) return r.err.message;
    this.set({ adminInfo: r.ok });
    return "AdminInfo retrieved";
  }

  // Server status.

  async sendHeartbeatAndGetServerStatus(): Promise<void> {
    const scope: GameListScope = this.state.game ? "None" : this.isAdmin ? "All" : "Active";
    this.updateServerStatusOnSuccess(await this.invoke("heartbeat", { scope }));
  }

  private handleUpdatedServerStatus(status: LobbyStatus): void {
    const active = status.runningGames;
    this.set({
      ownGames: status.ownGames,
      activeGames: active,
      activeGamesWithOpenSeats: active.filter((g) => canBeJoined(g)),
      activeGamesWithoutOpenSeats: active.filter((g) => !canBeJoined(g)),
      scheduledGames: status.scheduledGames,
      recentlySeenUsers: new Map(status.recentlySeenUsers.map((u) => [u.userId, u])),
    });
  }

  // Entering and leaving games.

  /** `Client.LoadGame`: takes the game's participation and starts following its views. */
  private async loadGame(info: GameInitInfo): Promise<string | null> {
    const current = this.state.game;
    if (current && current.gameId === info.gameId) {
      this.set({ game: { ...current, gameName: info.gameName, participation: new Participation(info.participation), phaseControl: info.phaseControl } });
      return null;
    }
    return this.enterGame(info.gameId, info);
  }

  private async enterGame(gameId: string, info: GameInitInfo | null): Promise<string | null> {
    this.leaveSource();
    const source = new WebSocketSource(this.conn, gameId);
    const game: CurrentGame = {
      gameId,
      gameName: info?.gameName ?? null,
      participation: new Participation(info?.participation),
      settings: defaultSettings(),
      phaseControl: info?.phaseControl ?? null,
      view: null,
      source,
    };
    this.set({ game });
    this.viewUnsubscribe = source.subscribe((view) => {
      const g = this.state.game;
      if (g && g.source === source) this.set({ game: { ...g, view } });
    });
    // After a dropped connection, take the participation from the rejoin: joins, leaves, kicks
    // and seat changes made while the socket was down sent this tab no notification. A user
    // who was kicked or left meanwhile goes back to the lobby, as with a removeUser.
    source.onRejoin((rejoined) => {
      if (this.state.game?.source !== source) return;
      void (rejoined ? this.loadGame(rejoined) : this.exitGame());
    });
    try {
      const view = await source.load();
      const g = this.state.game;
      if (g && g.source === source && g.view == null) this.set({ game: { ...g, view } });
      if (!info) {
        // A reconnect by URL: the participation comes from the game state.
        const r = await this.conn.call("gameState", { game: gameId });
        const g2 = this.state.game;
        if (!failed(r) && g2 && g2.source === source) {
          this.set({ game: { ...g2, gameName: r.ok.gameName, participation: new Participation(r.ok.participation), phaseControl: r.ok.phaseControl } });
        }
      }
      return null;
    } catch (e) {
      this.leaveSource();
      this.set({ game: null });
      return String(e);
    }
  }

  private leaveSource(): void {
    this.viewUnsubscribe?.();
    this.viewUnsubscribe = undefined;
    this.state.game?.source.dispose();
  }

  /** `Client.ExitGame`. */
  async exitGame(): Promise<void> {
    this.leaveSource();
    this.set({ game: null, reseatRequested: false });
    await this.sendHeartbeatAndGetServerStatus();
  }

  private async handleRemoveUser(userId: number, kick: boolean): Promise<void> {
    if (!this.state.game) return;
    if (userId === this.userId) await this.exitGame();
    else this.inGame((g) => g.participation.removeUser(userId, kick));
  }
}

let client: Promise<LobbyClient> | null = null;

/** The one lobby client of the page, with its lobby data and skin loaded. */
export function lobbyClient(): Promise<LobbyClient> {
  client ??= Promise.all([loadLobby(), loadSkin()]).then(([lobby, skin]) => new LobbyClient(lobby, skin));
  return client;
}
