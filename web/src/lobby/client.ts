// The lobby half of the C# Client (treachery-online/Client/Client.cs): the connection, the
// logged-in user, the server status (game lists), and the game the user is in, with its
// participation, settings and latest view. The game screen itself renders views from a
// WebSocketSource (state/source.ts); this store only says which screen of Main.razor to show and
// holds what those screens read beside the view.
//
// Every Request* method returns what the C# method returns: an error text for the screen, or
// empty/null on success. Texts come from Skin.Describe(ErrorType) (the lobby data).
import { Connection, type CallResult, type GameInitInfo, type LoginInfo } from "../state/connection";
import { WebSocketSource, type GameEventDraft } from "../state/source";
import type { GameView } from "../generated/GameView";
import { PhaseValue, type Phase } from "../generated/enums";
import { loadSkin, type Skin } from "../skin";
import { loadLobby, type Lobby } from "./data";
import { Participation } from "./participation";
import {
  type AdminInfo, canBeJoined, defaultSettings, type GameInfo, GameListScope, type GameSettings, type IntDict, type LoggedInUserInfo,
  type ParticipationJson, type PhaseControlInfo, type ScheduledGameInfo, type ServerInfo, type ServerStatus,
} from "./types";

/** `IGameService.HeartbeatDelay`. */
export const HEARTBEAT_DELAY = 6000;

const AUTH_TOKEN_SETTING = "treachery.online;auth.token";

const ErrorType = { UserNotFound: 1, InvalidGameEvent: 23 } as const;

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
  loginInfo: LoginInfo | null;
  ownGames: GameInfo[];
  activeGames: GameInfo[];
  activeGamesWithOpenSeats: GameInfo[];
  activeGamesWithoutOpenSeats: GameInfo[];
  scheduledGames: ScheduledGameInfo[];
  recentlySeenUsers: Map<number, LoggedInUserInfo>;
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
    connected: false, serverInfo: null, loginInfo: null, ownGames: [], activeGames: [], activeGamesWithOpenSeats: [],
    activeGamesWithoutOpenSeats: [], scheduledGames: [], recentlySeenUsers: new Map(), game: null, reseatRequested: false,
    adminInfo: null, version: 0,
  };
  private readonly listeners = new Set<() => void>();
  private storedPassword: string | null = null;
  private heartbeatTimer: number | undefined;
  private viewUnsubscribe: (() => void) | undefined;
  readonly conn: Connection;
  /** `Client.IsAdmin`: set by the admin page, which asks for every game. */
  isAdmin = false;

  constructor(readonly lobby: Lobby, readonly skin: Skin, conn?: Connection) {
    this.conn = conn ?? new Connection();
    this.conn.on("JoinGame", (userId, userName, seat) => this.inGame((g) => g.participation.addPlayer(userId as number, userName as string, seat as number)));
    this.conn.on("ObserveGame", (userId, userName) => this.inGame((g) => g.participation.addObserver(userId as number, userName as string)));
    this.conn.on("SetOrUnsetHost", (userId) => this.inGame((g) => g.participation.setOrUnsetHost(userId as number)));
    this.conn.on("OpenOrCloseSeats", (seats) => this.inGame((g) => (seats as number[]).forEach((s) => g.participation.openOrCloseSeat(s))));
    this.conn.on("AssignSeats", (assignment) => this.inGame((g) => g.participation.assignSeats(assignment as IntDict<number>)));
    this.conn.on("BotSpeed", (speed) => this.inGame((g) => { g.participation.botsSpeed = speed as number; }));
    this.conn.on("UpdateSettings", (settings) => this.inGame((g) => { g.settings = settings as GameSettings; }));
    this.conn.on("PhaseControl", (info) => this.inGame((g) => { g.phaseControl = info as PhaseControlInfo; }));
    this.conn.on("RemoveUser", (userId, kick) => { void this.handleRemoveUser(userId as number, kick as boolean); });
    this.conn.on("LoadGame", (info) => { void this.loadGame(info as GameInitInfo); });
    this.conn.onOpen(() => { this.set({ connected: true }); });
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

  get loggedIn(): boolean { return this.state.loginInfo != null; }
  get userId(): number { return this.state.loginInfo?.UserId ?? -1; }
  get userToken(): string | null { return this.state.loginInfo?.Token ?? null; }
  get inGameNow(): boolean { return this.state.game != null; }
  get isHost(): boolean { return this.state.game?.participation.isHost(this.userId) ?? false; }
  get isObserver(): boolean { return this.state.game?.participation.isObserver(this.userId) ?? false; }

  /** `Client.UserStatus`. */
  userStatus(): number {
    return this.state.recentlySeenUsers.get(this.userId)?.Status ?? 0;
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
    const start = Date.parse(info.ScheduledMaintenance);
    const now = Date.now();
    const hoursUntil = (start - now) / 3_600_000;
    const g = this.state.game;
    return start + 15 * 60_000 > now
      && ((g != null && hoursUntil < 6 && PhaseValue[phaseOf(g.view)] <= PhaseValue.AwaitingPlayers) || hoursUntil < 1);
  }

  // Start (Client.Start).

  async start(userToken?: string, gameId?: string): Promise<void> {
    await this.conn.open();
    this.set({ connected: true });
    await this.connect();
    if (userToken && gameId) {
      const r = await this.conn.call<LoginInfo>("GetLoginInfo", userToken);
      if (r.Success) {
        this.storedPassword = null;
        this.set({ loginInfo: r.Contents });
        await this.enterGame(gameId, null);
      }
    } else {
      await this.tryResumeSavedLogin();
    }
    this.scheduleHeartbeat();
  }

  private async connect(): Promise<void> {
    const r = await this.conn.call<ServerInfo>("Connect");
    if (r.Success) this.set({ serverInfo: r.Contents });
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

  /** `Client.Invoke`: a call that logs in again once with the stored password if the server forgot the user. */
  private async invoke<T>(method: string, ...args: unknown[]): Promise<CallResult<T>> {
    let r = await this.conn.call<T>(method, ...args);
    const name = this.state.loginInfo?.UserName;
    if (!r.Success && r.Error === ErrorType.UserNotFound && this.storedPassword != null && name != null) {
      if ((await this.requestLogin(name, this.storedPassword)).Success) r = await this.conn.call<T>(method, ...args);
    }
    return r;
  }

  private describe(r: CallResult): string {
    return this.lobby.describeError(r.Error);
  }

  // Authentication.

  async requestCreateUser(userName: string, hashedPassword: string, email: string | null, playerName: string): Promise<CallResult<LoginInfo>> {
    const r = await this.conn.call<LoginInfo>("RequestCreateUser", userName, hashedPassword, email, playerName);
    if (r.Success) {
      this.storedPassword = hashedPassword;
      this.set({ loginInfo: r.Contents });
    }
    return r;
  }

  async requestLogin(userName: string, hashedPassword: string): Promise<CallResult<LoginInfo>> {
    const r = await this.conn.call<LoginInfo>("RequestLogin", this.lobby.data.latestVersion, userName, hashedPassword);
    if (r.Success) {
      this.storedPassword = hashedPassword;
      this.set({ loginInfo: r.Contents });
      await this.sendHeartbeatAndGetServerStatus();
      return r;
    }
    this.storedPassword = null;
    this.set({ loginInfo: null });
    return r;
  }

  logout(): void {
    this.storedPassword = null;
    localStorage.setItem(AUTH_TOKEN_SETTING, "");
    this.set({ loginInfo: null });
  }

  /** Keeps the session token so a reload doesn't ask for credentials. */
  rememberLogin(keep: boolean): void {
    localStorage.setItem(AUTH_TOKEN_SETTING, keep && this.state.loginInfo ? this.state.loginInfo.Token : "");
  }

  private async tryResumeSavedLogin(): Promise<boolean> {
    const saved = localStorage.getItem(AUTH_TOKEN_SETTING);
    if (!saved) return false;
    const r = await this.conn.call<LoginInfo>("GetLoginInfo", saved);
    if (!r.Success) {
      localStorage.setItem(AUTH_TOKEN_SETTING, "");
      return false;
    }
    this.set({ loginInfo: r.Contents });
    await this.sendHeartbeatAndGetServerStatus();
    return true;
  }

  requestPasswordReset(usernameOrEmail: string): Promise<CallResult> {
    return this.conn.call("RequestPasswordReset", usernameOrEmail);
  }

  async requestSetPassword(userName: string, token: string, newHashedPassword: string): Promise<CallResult<LoginInfo>> {
    const r = await this.conn.call<LoginInfo>("RequestSetPassword", userName, token, newHashedPassword);
    if (r.Success) {
      this.storedPassword = newHashedPassword;
      this.set({ loginInfo: r.Contents });
    }
    return r;
  }

  async requestUpdateUserInfo(hashedPassword: string, email: string, playerName: string): Promise<string | null> {
    if (!this.loggedIn) return "Not logged in";
    const r = await this.invoke<LoginInfo>("RequestUpdateUserInfo", this.userToken, hashedPassword, playerName, email);
    if (!r.Success) return this.describe(r);
    this.storedPassword = hashedPassword;
    this.set({ loginInfo: r.Contents });
    return null;
  }

  async requestSetUserStatus(status: number): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    return this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestSetUserStatus", this.userToken, status));
  }

  // Game management.

  async requestCreateGame(name: string, password: string, stateData: string | null = null, skinData: string | null = null): Promise<string | null> {
    if (!this.loggedIn) return "Not logged in";
    const r = await this.invoke<GameInitInfo>("RequestCreateGame", name, this.userToken, password, stateData ?? "", skinData ?? "");
    if (r.Success && r.Contents) {
      const message = await this.loadGame(r.Contents);
      if (message != null) return message;
    }
    return r.Success ? null : r.Error === ErrorType.InvalidGameEvent ? r.ErrorDetails : this.describe(r);
  }

  async requestScheduleGame(dateTime: string, ruleset: number | null, numberOfPlayers: number, maximumTurns: number, allowedFactions: number[], asyncPlay: boolean): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestScheduleGame", this.userToken, dateTime, ruleset, numberOfPlayers, maximumTurns, allowedFactions, asyncPlay));
  }

  async requestCancelGame(scheduledGameId: string): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestCancelGame", this.userToken, scheduledGameId));
  }

  async requestCloseGame(gameId: string): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestCloseGame", this.userToken, gameId));
  }

  private updateServerStatusOnSuccess(r: CallResult<ServerStatus>): string {
    if (!r.Success || !r.Contents) return this.describe(r);
    this.handleUpdatedServerStatus(r.Contents);
    return "";
  }

  async requestUpdateSettings(gameId: string, settings: GameSettings): Promise<string> {
    const r = await this.invoke("RequestUpdateSettings", this.userToken, gameId, settings);
    return r.Success ? "" : this.describe(r);
  }

  async requestJoinGame(gameId: string, password: string, seat: number): Promise<string | null> {
    const r = await this.invoke<GameInitInfo>("RequestJoinGame", this.userToken, gameId, password, seat);
    if (!r.Success || !r.Contents) return this.describe(r);
    return this.loadGame(r.Contents);
  }

  async requestSubscribeGame(scheduledGameId: string, subscription: number): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestSubscribeGame", this.userToken, scheduledGameId, subscription));
  }

  async requestObserveGame(gameId: string, password: string): Promise<string | null> {
    const r = await this.invoke<GameInitInfo>("RequestObserveGame", this.userToken, gameId, password);
    if (!r.Success || !r.Contents) return this.describe(r);
    return this.loadGame(r.Contents);
  }

  /** The C# methods that answer `Describe(Error)` even on success, which is the empty text. */
  private async gameCall(method: string, ...args: unknown[]): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    const g = this.state.game;
    if (!g) return "Not in game";
    return this.describe(await this.invoke(method, this.userToken, g.gameId, ...args));
  }

  requestSetOrUnsetHost(userId: number): Promise<string> { return this.gameCall("RequestSetOrUnsetHost", userId); }
  requestOpenOrCloseSeat(seat: number): Promise<string> { return this.gameCall("RequestOpenOrCloseSeat", seat); }
  requestKick(userId: number): Promise<string> { return this.gameCall("RequestKick", userId); }
  requestSetBotSpeed(speed: number): Promise<string> { return this.gameCall("RequestSetBotSpeed", speed); }

  requestReseat(): void {
    this.set({ reseatRequested: true });
  }

  async requestLeaveGame(): Promise<string> {
    return this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestLeaveGame", this.userToken, this.state.game?.gameId ?? null));
  }

  async requestAssignSeats(assignment: IntDict<number>): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    if (!this.state.game) return "Not in game";
    this.set({ reseatRequested: false });
    return this.gameCall("RequestAssignSeats", assignment);
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
    if (event.$type === "EstablishPlayers" && (event.Settings as GameSettings | undefined)?.AutoOpenEmptySeats) {
      const players = this.state.game?.view?.players ?? [];
      const seated = this.state.game?.participation.seatedPlayers.values() ?? [];
      for (const p of players.filter((p) => !seated.includes(p.seat))) await this.requestOpenOrCloseSeat(p.seat);
    }
    return result;
  }

  // Admin (Client.Admin*).

  private async adminCall(method: string, ...args: unknown[]): Promise<string> {
    const r = await this.invoke<string>(method, this.userToken, ...args);
    return r.Success && r.Contents != null ? r.Contents : this.describe(r);
  }

  async adminUpdateMaintenance(date: string): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    return this.adminCall("AdminUpdateMaintenance", date);
  }

  async adminPersistState(): Promise<string> {
    if (!this.loggedIn) return "Not logged in";
    // As Client.AdminPersistState, which refuses outside a game (the admin page is never in one).
    if (!this.state.game) return "Not in game";
    return this.adminCall("AdminPersistState");
  }

  adminRestoreState(): Promise<string> { return this.adminCall("AdminRestoreState"); }
  adminCloseGame(gameId: string): Promise<string> { return this.adminCall("AdminCloseGame", gameId); }
  adminCancelGame(scheduledGameId: string): Promise<string> { return this.adminCall("AdminCancelGame", scheduledGameId); }
  adminDeleteUser(userId: number): Promise<string> { return this.adminCall("DeleteUser", userId); }

  async adminDownloadGame(gameId: string): Promise<string> {
    const r = await this.invoke<string>("AdminDownloadGame", this.userToken, gameId);
    if (!r.Success || r.Contents == null) return this.describe(r);
    const url = URL.createObjectURL(new Blob([r.Contents], { type: "application/json" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `savegame-${gameId}.json`;
    a.click();
    URL.revokeObjectURL(url);
    return "Savegame downloaded";
  }

  async getAdminInfo(): Promise<string> {
    const r = await this.invoke<AdminInfo>("GetAdminInfo", this.userToken);
    if (r.Success && r.Contents) {
      this.set({ adminInfo: r.Contents });
      return "AdminInfo retrieved";
    }
    return this.describe(r);
  }

  // Server status.

  async sendHeartbeatAndGetServerStatus(): Promise<void> {
    const scope = this.state.game ? GameListScope.None : this.isAdmin ? GameListScope.All : GameListScope.Active;
    this.updateServerStatusOnSuccess(await this.invoke<ServerStatus>("RequestHeartbeat", this.userToken, scope));
  }

  private handleUpdatedServerStatus(status: ServerStatus): void {
    const awaiting = this.lobby.value("Phase", "AwaitingPlayers");
    const active = status.RunningGames;
    this.set({
      ownGames: status.OwnGames,
      activeGames: active,
      activeGamesWithOpenSeats: active.filter((g) => canBeJoined(g, g.Phase === awaiting)),
      activeGamesWithoutOpenSeats: active.filter((g) => !canBeJoined(g, g.Phase === awaiting)),
      scheduledGames: status.ScheduledGames,
      recentlySeenUsers: new Map(status.RecentlySeenUsers.map((u) => [u.Id, u])),
    });
  }

  // Entering and leaving games.

  /** `Client.LoadGame`: takes the game's participation and starts following its views. */
  private async loadGame(info: GameInitInfo): Promise<string | null> {
    const current = this.state.game;
    if (current && current.gameId === info.GameId) {
      this.set({ game: { ...current, gameName: info.GameName ?? current.gameName, participation: new Participation(info.Participation as ParticipationJson), phaseControl: (info.PhaseControl as PhaseControlInfo | null) ?? null } });
      return null;
    }
    return this.enterGame(info.GameId, info);
  }

  private async enterGame(gameId: string, info: GameInitInfo | null): Promise<string | null> {
    this.leaveSource();
    const source = new WebSocketSource(this.conn, this.userToken!, gameId);
    const game: CurrentGame = {
      gameId,
      gameName: info?.GameName ?? null,
      participation: new Participation(info?.Participation as ParticipationJson | undefined),
      settings: defaultSettings(),
      phaseControl: (info?.PhaseControl as PhaseControlInfo | null) ?? null,
      view: null,
      source,
    };
    this.set({ game });
    this.viewUnsubscribe = source.subscribe((view) => {
      const g = this.state.game;
      if (g && g.source === source) this.set({ game: { ...g, view } });
    });
    try {
      const view = await source.load();
      const g = this.state.game;
      if (g && g.source === source && g.view == null) this.set({ game: { ...g, view } });
      if (!info) {
        // A reconnect by URL: the participation comes from the game state.
        const r = await this.conn.call<GameInitInfo>("RequestGameState", this.userToken, gameId);
        const g2 = this.state.game;
        if (r.Success && r.Contents && g2 && g2.source === source) {
          this.set({ game: { ...g2, gameName: r.Contents.GameName, participation: new Participation(r.Contents.Participation as ParticipationJson), phaseControl: (r.Contents.PhaseControl as PhaseControlInfo | null) ?? null } });
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
