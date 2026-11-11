// OtherComponents/OwnGamesComponent.razor: your games (started and scheduled), creating or
// loading a game, scheduling one, and your account.
import { useState } from "react";
import type { Faction } from "../../generated/enums";
import { hashPassword } from "../../state/connection";
import { useLobby } from "../../lobby/context";
import { isNullOrEmpty, isNullOrWhiteSpace } from "../../lobby/text";
import { type GameInfo, youAreIn, yourCurrentSeat } from "../../lobby/types";
import { Checkbox } from "../Checkbox";
import { CancelIcon, PlayIcon, TrashIcon } from "./Icons";
import { activityColor, Subscribers, useRedrawCycle } from "./RunningGamesComponent";
import { UserStatusComponent } from "./SubscribeComponent";
import { CommandButton, DateTimePicker, FactionShown, SelectFactions, SelectNumber, shortDate, toDateTimeOffset } from "./widgets";
import { Select } from "../Select";

type State = "List" | "Create" | "Schedule";

const MAX_SAVE_GAME_SIZE = 4194304;

export function OwnGamesComponent() {
  const [client, lobbyState] = useLobby();
  const { lobby } = client;
  const login = lobbyState.loginInfo!;
  const allFactions = lobby.data.availableFactions as Faction[];
  const allRulesets = lobby.data.availableRulesets;
  const faction = (f: number) => lobby.name<Faction>("Faction", f);
  useRedrawCycle(3200);

  const [state, setState] = useState<State>("List");

  const [createGameHasPassword, setCreateGameHasPassword] = useState(false);
  const [createGameName, setCreateGameName] = useState(`${login.PlayerName}'s Game`);
  const [createGamePassword, setCreateGamePassword] = useState("");
  const [createError, setCreateError] = useState("");
  const [loading, setLoading] = useState(false);

  const [scheduleDate, setScheduleDate] = useState(() => new Date());
  const [scheduleRuleset, setScheduleRuleset] = useState("AllExpansionsAdvancedGame");
  const [schedulePlayers, setSchedulePlayers] = useState(6);
  const [scheduleTurns, setScheduleTurns] = useState(10);
  const [scheduleFactions, setScheduleFactions] = useState<Faction[]>(() => lobby.members("Faction").filter((m) => m.name !== "None").map((m) => m.name as Faction));
  const [scheduleAsync, setScheduleAsync] = useState(false);
  const [scheduleError] = useState("");

  const [editUser, setEditUser] = useState(false);
  const [deleteUser, setDeleteUser] = useState(false);
  const [playerName, setPlayerName] = useState(login.PlayerName);
  const [email, setEmail] = useState(login.Email ?? "");
  const [password1, setPassword1] = useState("");
  const [password2, setPassword2] = useState("");
  const [changeUserInfoMessage, setChangeUserInfoMessage] = useState<string | null>(null);

  const getUserInfoFromClient = () => {
    const l = client.getState().loginInfo;
    setPlayerName(l?.PlayerName ?? "");
    setEmail(l?.Email ?? "");
    setPassword1("");
    setPassword2("");
  };

  const myGames = [...lobbyState.ownGames].sort((a, b) => Date.parse(b.LastActivity ?? "") - Date.parse(a.LastActivity ?? "") || 0);
  const myScheduledGames = lobbyState.scheduledGames.filter((g) => g.CreatorUserId === client.userId).sort((a, b) => Date.parse(b.DateTime) - Date.parse(a.DateTime));

  const scheduleGame = async () => {
    await client.requestScheduleGame(toDateTimeOffset(scheduleDate), lobby.value("Ruleset", scheduleRuleset), schedulePlayers, scheduleTurns,
      scheduleFactions.map((f) => lobby.value("Faction", f)), scheduleAsync);
    setScheduleDate(new Date());
    setScheduleRuleset("AllExpansionsAdvancedGame");
    setSchedulePlayers(6);
    setScheduleTurns(10);
    setScheduleFactions(lobby.members("Faction").filter((m) => m.name !== "None").map((m) => m.name as Faction));
    setScheduleAsync(false);
    setState("List");
  };

  const createNewGame = async () => {
    setCreateError((await client.requestCreateGame(createGameName, createGameHasPassword ? createGamePassword : "")) ?? "");
  };

  const loadAndCreateGame = async (file: File | undefined) => {
    setCreateError("");
    setLoading(true);
    if (!file) return;
    try {
      const data = await file.text();
      if (data.length > 0) {
        if (data.length > MAX_SAVE_GAME_SIZE) setCreateError("File is too big");
        else setCreateError((await client.requestCreateGame(createGameName, createGamePassword, data, null)) ?? "");
      }
    } catch (e) {
      setCreateError(`Error loading game (${String(e)})`);
    }
  };

  const updateUserInfo = async () => {
    let message: string | null = null;
    if (!isNullOrEmpty(password1) && password1 !== password2) message = "Passwords do not match";
    else if (!isNullOrWhiteSpace(email) && (!email.includes("@") || !email.includes("."))) message = "Enter a valid e-mail address, or leave it empty";
    setChangeUserInfoMessage(message);
    if (message != null) return;
    await client.requestUpdateUserInfo(isNullOrEmpty(password1) ? "" : await hashPassword(password1), email, playerName);
    getUserInfoFromClient();
    setEditUser(false);
  };

  const deleteUserInfo = async () => {
    await client.adminDeleteUser(client.userId);
    setDeleteUser(false);
    client.logout();
  };

  const active = (s: State) => (s === state ? "active" : "");
  const factionIcons = (fs: number[]) => fs.map((f, i) => <FactionShown key={i} faction={faction(f)} width={18} noShadow />);

  const gameRow = (av: GameInfo) => (
    <tr key={av.GameId} className="small table-dark">
      <td className="w-25">{av.Name}</td>
      <td className="w-25" style={activityColor(av, 20)}>
        <div>{lobby.label("Ruleset", av.Ruleset)}, Turn {av.Turn}: {lobby.label("MainPhase", av.MainPhase)}
          <br />
          {factionIcons((av.FactionsInPlay ?? []).filter((f) => f !== 0))}
        </div>
      </td>
      <td className="w-25">{av.LastActivity ? shortDate(av.LastActivity) : ""}</td>
      <td className="w-25" style={{ textAlign: "right" }}>
        {youAreIn(av, client.userId) && (
          <CommandButton small onClick={() => client.requestJoinGame(av.GameId, "", yourCurrentSeat(av, client.userId))} marginClass="m-1" title="Resume game"><PlayIcon /></CommandButton>
        )}
        <CommandButton small onClick={async () => setCreateError(await client.requestCloseGame(av.GameId))} marginClass="m-1" title="Delete game"><TrashIcon /></CommandButton>
      </td>
    </tr>
  );

  return (
    <>
      <div className="card mb-2 shadow-lg" style={{ animation: "fadeIn 1s" }}>
        <div className="card-header">Your games</div>
        <div className="card-body">
          <ul className="nav nav-tabs mb-2">
            <li className="nav-item"><button className={`nav-link ${active("List")}`} aria-current="page" onClick={() => setState("List")}>Your games</button></li>
            <li className="nav-item"><button className={`nav-link ${active("Create")}`} onClick={() => setState("Create")}>Create or load a game</button></li>
            <li className="nav-item"><button className={`nav-link ${active("Schedule")}`} onClick={() => setState("Schedule")}>Schedule a game</button></li>
          </ul>

          {state === "List" && (
            <>
              {myGames.length === 0 && myScheduledGames.length === 0 && <div className="text-info mb-2">No games yet...</div>}
              {myGames.length > 0 && (
                <>
                  <div className="text-info mb-2">Started games</div>
                  <table className="table table-sm">
                    <thead>
                      <tr className="small table-dark">
                        <th scope="col">Name</th>
                        <th scope="col">Info</th>
                        <th scope="col">Last action</th>
                        <th scope="col" style={{ textAlign: "right" }}>Actions</th>
                      </tr>
                    </thead>
                    <tbody>{myGames.map(gameRow)}</tbody>
                  </table>
                </>
              )}
              {myScheduledGames.length > 0 && (
                <>
                  <div className="text-info mb-2">Scheduled games</div>
                  <table className="table table-sm">
                    <thead>
                      <tr className="small table-dark">
                        <th scope="col">Game info</th>
                        <th scope="col">Subscribed players</th>
                        <th scope="col">Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      {myScheduledGames.map((game) => (
                        <tr key={game.ScheduledGameId} className="small table-dark">
                          <td className="w-20">{shortDate(game.DateTime)}</td>
                          <td className="w-40">
                            <div>{lobby.label("Ruleset", game.Ruleset)}, {game.NumberOfPlayers} players, {game.MaximumTurns} turns
                              {" "}{game.AllowedFactionsInPlay.length === 12 ? <span>(all factions)</span> : factionIcons(game.AllowedFactionsInPlay)}
                            </div>
                          </td>
                          <td className="w-20" style={{ textAlign: "right" }}><Subscribers game={game} size={18} /></td>
                          <td className="w-20"><CommandButton small onClick={() => client.requestCancelGame(game.ScheduledGameId)}><CancelIcon /></CommandButton></td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </>
              )}
            </>
          )}

          {state === "Create" && (
            <>
              <input type="text" className="form-control" value={createGameName} onChange={(e) => setCreateGameName(e.target.value)} maxLength={64} placeholder="Game name" />
              <Checkbox value={createGameHasPassword} onChange={setCreateGameHasPassword}>Require a password to join?</Checkbox>
              {createGameHasPassword && (
                <input type="text" className="form-control" value={createGamePassword} onChange={(e) => setCreateGamePassword(e.target.value)} maxLength={24} placeholder="Password" />
              )}
              <div className="text-danger small">{createError}</div>
              <CommandButton marginClass="mt-1" onClick={createNewGame}>Create new game</CommandButton>{" "}
              <label className="btn btn-primary mt-1" {...{ disabled: loading }}>Load game from file
                {loading && (
                  <div className="spinner-border spinner-border-sm" role="status">
                    <span className="visually-hidden">Loading...</span>
                  </div>
                )}
                <input hidden type="file" id="fileInput" onChange={(e) => void loadAndCreateGame(e.target.files?.[0])} />
              </label>
            </>
          )}

          {state === "Schedule" && (
            <>
              <DateTimePicker value={scheduleDate} onChange={setScheduleDate} />
              <SelectNumber value={schedulePlayers} onChange={setSchedulePlayers} min={2} max={lobby.data.maximumPlayers} label="Number of players:" />
              <SelectNumber value={scheduleTurns} onChange={setScheduleTurns} min={1} max={lobby.data.maximumTurns} label="Maximum number of turns:" />
              <SelectFactions values={allFactions} selected={scheduleFactions} onChange={setScheduleFactions} imageWidth={40} label="Factions in play:" />
              <Select required label="Ruleset:" value={scheduleRuleset} onChange={setScheduleRuleset}
                               options={allRulesets.map((r) => ({ value: r as string, label: lobby.labelOf("Ruleset", r) }))} />
              <Checkbox value={scheduleAsync} onChange={setScheduleAsync}>Async play (send e-mail updates)</Checkbox>
              <div className="text-danger small">{scheduleError}</div>
              <CommandButton marginClass="mt-1" onClick={scheduleGame}>Schedule game</CommandButton>{" "}
              <a className="btn btn-dark btn-sm mt-1 p-2 text-start p-0 mt-0 mb-0" href="https://discord.gg/rRGEfn4" target="_blank" title="Find players on Dune Discord">
                <svg style={{ marginRight: "2px" }} width="18" height="18" viewBox="0 -28.5 256 256" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid">
                  <g>
                    <path d="M216.856339,16.5966031 C200.285002,8.84328665 182.566144,3.2084988 164.041564,0 C161.766523,4.11318106 159.108624,9.64549908 157.276099,14.0464379 C137.583995,11.0849896 118.072967,11.0849896 98.7430163,14.0464379 C96.9108417,9.64549908 94.1925838,4.11318106 91.8971895,0 C73.3526068,3.2084988 55.6133949,8.86399117 39.0420583,16.6376612 C5.61752293,67.146514 -3.4433191,116.400813 1.08711069,164.955721 C23.2560196,181.510915 44.7403634,191.567697 65.8621325,198.148576 C71.0772151,190.971126 75.7283628,183.341335 79.7352139,175.300261 C72.104019,172.400575 64.7949724,168.822202 57.8887866,164.667963 C59.7209612,163.310589 61.5131304,161.891452 63.2445898,160.431257 C105.36741,180.133187 151.134928,180.133187 192.754523,160.431257 C194.506336,161.891452 196.298154,163.310589 198.110326,164.667963 C191.183787,168.842556 183.854737,172.420929 176.223542,175.320965 C180.230393,183.341335 184.861538,190.991831 190.096624,198.16893 C211.238746,191.588051 232.743023,181.531619 254.911949,164.955721 C260.227747,108.668201 245.831087,59.8662432 216.856339,16.5966031 Z M85.4738752,135.09489 C72.8290281,135.09489 62.4592217,123.290155 62.4592217,108.914901 C62.4592217,94.5396472 72.607595,82.7145587 85.4738752,82.7145587 C98.3405064,82.7145587 108.709962,94.5189427 108.488529,108.914901 C108.508531,123.290155 98.3405064,135.09489 85.4738752,135.09489 Z M170.525237,135.09489 C157.88039,135.09489 147.510584,123.290155 147.510584,108.914901 C147.510584,94.5396472 157.658606,82.7145587 170.525237,82.7145587 C183.391518,82.7145587 193.761324,94.5189427 193.539891,108.914901 C193.539891,123.290155 183.391518,135.09489 170.525237,135.09489 Z" fill="white" fillRule="nonzero" />
                  </g>
                </svg>
                {" "}Find players on Dune Discord
              </a>
            </>
          )}
        </div>
      </div>

      <div className="card mb-2 shadow-lg" style={{ animation: "fadeIn 1s" }}>
        <div className="card-header">Your account</div>
        <div className="card-body">
          <UserStatusComponent />
          <div className="row">
            <label htmlFor="userName" className="col-4 col-form-label">Username</label>
            <div className="col-8">
              <input type="text" readOnly className="form-control-plaintext" id="userName" value={login.UserName} />
            </div>
          </div>
          {deleteUser ? (
            <>
              <div className="form-label alert-danger">Are your sure you wish to delete your account and all your games?</div>
              <button className="btn btn-secondary btn-sm me-1" onClick={() => setDeleteUser(false)}>Cancel</button>{" "}
              <CommandButton className="btn-danger" onClick={deleteUserInfo} small>Yes, delete my account</CommandButton>
            </>
          ) : editUser ? (
            <>
              <div className="row">
                <label htmlFor="playerName" className="col-4 col-form-label">Player name</label>
                <div className="col-8"><input type="text" className="form-control" id="playerName" value={playerName} onChange={(e) => setPlayerName(e.target.value)} style={{ fontSize: "20px" }} /></div>
              </div>
              <div className="row">
                <label htmlFor="email" className="col-4 col-form-label">E-mail address</label>
                <div className="col-8"><input type="text" className="form-control" id="email" value={email} onChange={(e) => setEmail(e.target.value)} /></div>
              </div>
              <div className="row">
                <label htmlFor="password1" className="col-4 col-form-label">Password</label>
                <div className="col-8"><input type="password" className="form-control" id="password1" autoComplete="new-password" value={password1} onChange={(e) => setPassword1(e.target.value)} /></div>
              </div>
              <div className="row mb-1">
                <label htmlFor="password2" className="col-4 col-form-label">Repeat password</label>
                <div className="col-8"><input type="password" className="form-control" id="password2" autoComplete="new-password" value={password2} onChange={(e) => setPassword2(e.target.value)} /></div>
              </div>
              <div className="text-danger small mb-1">{changeUserInfoMessage}</div>
              <button className="btn btn-danger btn-sm me-1" onClick={() => setDeleteUser(true)}>Delete account</button>{" "}
              <button className="btn btn-secondary btn-sm me-1" onClick={() => { getUserInfoFromClient(); setEditUser(false); }}>Cancel</button>{" "}
              <CommandButton onClick={updateUserInfo} small>Apply changes</CommandButton>
            </>
          ) : (
            <>
              <div className="row">
                <label htmlFor="playerName" className="col-4 col-form-label">Player name</label>
                <div className="col-8"><input type="text" readOnly className="form-control-plaintext" id="playerName" value={playerName} style={{ fontSize: "20px" }} /></div>
              </div>
              <div className="row">
                <label htmlFor="email" className="col-4 col-form-label">E-mail address</label>
                <div className="col-8"><input type="text" readOnly className="form-control-plaintext" id="email" value={email} /></div>
              </div>
              <button className="btn btn-primary me-1" onClick={() => setEditUser(true)}>Change account info</button>{" "}
              <button className="btn btn-secondary" onClick={() => client.logout()}>Log out</button>
            </>
          )}
        </div>
      </div>
    </>
  );
}
