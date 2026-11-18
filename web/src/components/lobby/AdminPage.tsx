// Pages/Admin.razor: server info, the maintenance window, persisting and restoring, users, running
// and scheduled games, with a log of the answers.
import { useState } from "react";
import { useLobby } from "../../lobby/context";
import type { GameInfo, ScheduledGameInfo } from "../../lobby/types";
import { MainLayout } from "../MainLayout";
import { LoginComponent } from "./LoginComponent";
import { START_SCREEN_STYLE, WithLobby } from "./MainPage";
import { activityColor } from "./RunningGamesComponent";
import { DateTimePicker, longDate, toDateTimeOffset } from "./widgets";

function AdminScreen() {
  const [client, state] = useLobby();
  const [maintenance, setMaintenance] = useState<Date | null>(null);
  const [usernameFilter, setUsernameFilter] = useState("");
  const [timeFrom, setTimeFrom] = useState("");
  const [timeTo, setTimeTo] = useState("");
  const [messages, setMessages] = useState<string[]>([]);
  const log = (m: string) => setMessages((ms) => [...ms, m]);
  // DateTimePicker: DateTimeOffset.MinValue (no maintenance planned) shows the current time.
  const planned = state.serverInfo ? new Date(state.serverInfo.scheduledMaintenance) : null;
  const maintenanceDate = maintenance ?? (planned && planned.getFullYear() > 1 ? planned : new Date());

  if (!state.connected) {
    return (
      <div className="start-row start-row--centred" style={START_SCREEN_STYLE}>
        <div className="start-row__notice">Not Connected</div>
      </div>
    );
  }
  if (!state.account) {
    return (
      <div className="start-row start-row--centred" style={START_SCREEN_STYLE}>
        <div className="start-row__col start-row__col--side"></div>
        <div className="start-row__col start-row__col--main"><div className="start-row__center"><LoginComponent /></div></div>
        <div className="start-row__col start-row__col--side"></div>
      </div>
    );
  }

  const from = timeFrom ? new Date(timeFrom).getTime() : null;
  const to = timeTo ? new Date(timeTo).getTime() : null;
  const filtered = state.activeGames
    .filter((g) => usernameFilter.trim() === "" || g.creatorUserName.toLowerCase().includes(usernameFilter.toLowerCase()))
    .filter((g) => from == null || Date.parse(g.created) >= from)
    .filter((g) => to == null || Date.parse(g.created) <= to)
    .sort((a, b) => Date.parse(b.lastActivity ?? "") - Date.parse(a.lastActivity ?? ""));
  const scheduled = [...state.scheduledGames].sort((a, b) => Date.parse(b.starts) - Date.parse(a.starts));
  const admin = state.adminInfo;

  return (
    <div className="start-row" style={START_SCREEN_STYLE}>
      <div className="start-row__col start-row__col--margin"></div>
      <div className="start-row__col start-row__col--main">
        <div className="card admin-page__card">
          <div className="card__header">Server info</div>
          <div className="card__body">
            <h3>Info</h3>
            <button className="button button--primary admin-page__button" onClick={async () => log(await client.getAdminInfo())}>Get admin info</button>
            <h3>Maintenance window</h3>
            <DateTimePicker value={maintenanceDate} onChange={setMaintenance} />
            <button className="button button--primary admin-page__button" onClick={async () => log(await client.adminUpdateMaintenance(toDateTimeOffset(maintenanceDate)))}>Update</button>
            <h3>Persist data</h3>
            <button className="button button--primary admin-page__button" onClick={async () => log(await client.adminPersistState())}>Persist data</button>
            <h3>Restore persisted data</h3>
            <button className="button button--primary admin-page__button" onClick={async () => log(await client.adminRestoreState())}>Restore data</button>
          </div>
        </div>
        {admin && (
          <div className="card admin-page__card">
            <div className="card__header">Info</div>
            <div className="card__body">
              <p><strong>Users by User Token: </strong>{admin.sessions}</p>
              <p><strong>Games by Game Id: </strong>{admin.runningGames}</p>
              <p><strong>Total Users: </strong>{admin.users.length}</p>
              <p><strong>ConnectionInfo by UserId: </strong>{admin.usersInGames}</p>
              <table className="table table--small table--dark">
                <thead>
                  <tr className="admin-page__row">
                    <th>User id</th><th>Username</th><th>Player name</th><th>Email</th><th>Last login</th><th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {admin.users.map((u) => (
                    <tr key={u.userId} className="admin-page__row">
                      <td>{u.userId}</td>
                      <td>{u.userName}</td>
                      <td>{u.playerName}</td>
                      <td>{u.email}</td>
                      <td>{u.lastLogin && !u.lastLogin.startsWith("0001-01-01") ? longDate(u.lastLogin) : ""}</td>
                      <td><button className="button button--primary button--small admin-page__button" onClick={async () => log(await client.adminDeleteUser(u.userId))}>Delete</button></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
        <div className="card admin-page__card">
          <div className="card__header">Running games</div>
          <div className="card__body">
            <div className="admin-page__filters">
              <div className="admin-page__filter">
                <label className="label" htmlFor="gameUsernameFilter">Username</label>
                <input id="gameUsernameFilter" className="input input--small" value={usernameFilter} onChange={(e) => setUsernameFilter(e.target.value)} />
              </div>
              <div className="admin-page__filter">
                <label className="label" htmlFor="gameTimeFromFilter">Game time from</label>
                <input id="gameTimeFromFilter" type="datetime-local" className="input input--small" value={timeFrom} onChange={(e) => setTimeFrom(e.target.value)} />
              </div>
              <div className="admin-page__filter">
                <label className="label" htmlFor="gameTimeToFilter">Game time to</label>
                <input id="gameTimeToFilter" type="datetime-local" className="input input--small" value={timeTo} onChange={(e) => setTimeTo(e.target.value)} />
              </div>
            </div>
            <table className="table table--small table--dark">
              <thead>
                <tr className="admin-page__row">
                  <th>Created</th><th>Username</th><th>Last activity</th><th>Game</th><th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((g: GameInfo) => {
                  const style = activityColor(g, 20);
                  return (
                    <tr key={g.gameId} className="admin-page__row">
                      <td style={style}>{longDate(g.created)}</td>
                      <td style={style}>{g.creatorUserName}</td>
                      <td style={style}>{g.lastActivity ? longDate(g.lastActivity, true) : ""}</td>
                      <td style={style}>{g.name}</td>
                      <td>
                        <button className="button button--primary button--small admin-page__button" title="Download savegame" aria-label="Download savegame" onClick={async () => log(await client.adminDownloadGame(g.gameId))}>
                          <span className="oi oi-data-transfer-download" aria-hidden="true"></span>
                        </button>{" "}
                        <button className="button button--primary button--small admin-page__button" onClick={async () => log(await client.adminCloseGame(g.gameId))}>Close</button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
        <div className="card admin-page__card">
          <div className="card__header">Scheduled games</div>
          <div className="card__body">
            <table className="table table--small table--dark">
              <tbody>
                {scheduled.map((g: ScheduledGameInfo) => (
                  <tr key={g.scheduledGameId} className="admin-page__row">
                    <td className="admin-page__quarter">{longDate(g.starts, true)}</td>
                    <td className="admin-page__quarter">{g.creatorName}</td>
                    <td className="admin-page__quarter"><button className="button button--primary button--small admin-page__button" onClick={async () => log(await client.adminCancelGame(g.scheduledGameId))}>Cancel</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
      <div className="start-row__col start-row__col--margin">
        <div className="card admin-page__card">
          <div className="card__header">Log</div>
          <div className="card__body">{messages.map((m, i) => <div key={i}>{m}</div>)}</div>
        </div>
      </div>
      <div className="start-row__col start-row__col--margin"></div>
    </div>
  );
}

/** Route `/admin`. */
export function AdminPage() {
  return <WithLobby admin><MainLayout><AdminScreen /></MainLayout></WithLobby>;
}
