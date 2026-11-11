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
  const planned = state.serverInfo ? new Date(state.serverInfo.ScheduledMaintenance) : null;
  const maintenanceDate = maintenance ?? (planned && planned.getFullYear() > 1 ? planned : new Date());

  if (!state.connected) {
    return (
      <div className="row d-flex align-items-center" style={START_SCREEN_STYLE}>
        <div className="d-flex justify-content-center">Not Connected</div>
      </div>
    );
  }
  if (!state.loginInfo) {
    return (
      <div className="row d-flex align-items-center" style={START_SCREEN_STYLE}>
        <div className="col-sm-12 col-lg-3"></div>
        <div className="col-sm-12 col-lg-6"><div className="d-flex justify-content-center"><LoginComponent /></div></div>
        <div className="col-sm-12 col-lg-3"></div>
      </div>
    );
  }

  const from = timeFrom ? new Date(timeFrom).getTime() : null;
  const to = timeTo ? new Date(timeTo).getTime() : null;
  const filtered = state.activeGames
    .filter((g) => usernameFilter.trim() === "" || g.CreatorUsername.toLowerCase().includes(usernameFilter.toLowerCase()))
    .filter((g) => from == null || Date.parse(g.CreationDate) >= from)
    .filter((g) => to == null || Date.parse(g.CreationDate) <= to)
    .sort((a, b) => Date.parse(b.LastActivity ?? "") - Date.parse(a.LastActivity ?? ""));
  const scheduled = [...state.scheduledGames].sort((a, b) => Date.parse(b.DateTime) - Date.parse(a.DateTime));
  const admin = state.adminInfo;

  return (
    <div className="row" style={START_SCREEN_STYLE}>
      <div className="col-sm-12 col-lg-2"></div>
      <div className="col-sm-12 col-lg-6">
        <div className="card mb-2">
          <div className="card-header">Server info</div>
          <div className="card-body">
            <h3 className="h3">Info</h3>
            <button className="btn btn-primary me-1" onClick={async () => log(await client.getAdminInfo())}>Get admin info</button>
            <h3 className="h3">Maintenance window</h3>
            <DateTimePicker value={maintenanceDate} onChange={setMaintenance} />
            <button className="btn btn-primary me-1" onClick={async () => log(await client.adminUpdateMaintenance(toDateTimeOffset(maintenanceDate)))}>Update</button>
            <h3 className="h3">Persist data</h3>
            <button className="btn btn-primary me-1" onClick={async () => log(await client.adminPersistState())}>Persist data</button>
            <h3 className="h3">Restore persisted data</h3>
            <button className="btn btn-primary me-1" onClick={async () => log(await client.adminRestoreState())}>Restore data</button>
          </div>
        </div>
        {admin && (
          <div className="card mb-2">
            <div className="card-header">Info</div>
            <div className="card-body">
              <p><strong>Users by User Token: </strong>{admin.UsersByUserTokenCount}</p>
              <p><strong>Games by Game Id: </strong>{admin.GamesByGameIdCount}</p>
              <p><strong>Total Users: </strong>{admin.Users.length}</p>
              <p><strong>ConnectionInfo by UserId: </strong>{admin.ConnectionInfoByUserIdCount}</p>
              <table className="table table-sm">
                <thead>
                  <tr className="small table-dark">
                    <th>User id</th><th>Username</th><th>Player name</th><th>Email</th><th>Last login</th><th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {admin.Users.map((u) => (
                    <tr key={u.Id} className="small table-dark">
                      <td>{u.Id}</td>
                      <td>{u.Username}</td>
                      <td>{u.PlayerName}</td>
                      <td>{u.Email}</td>
                      <td>{u.LastLogin && !u.LastLogin.startsWith("0001-01-01") ? longDate(u.LastLogin) : ""}</td>
                      <td><button className="btn btn-primary btn-sm me-1" onClick={async () => log(await client.adminDeleteUser(u.Id))}>Delete</button></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
        <div className="card mb-2">
          <div className="card-header">Running games</div>
          <div className="card-body">
            <div className="row g-2 mb-3">
              <div className="col-sm-12 col-md-4">
                <label className="form-label" htmlFor="gameUsernameFilter">Username</label>
                <input id="gameUsernameFilter" className="form-control form-control-sm" value={usernameFilter} onChange={(e) => setUsernameFilter(e.target.value)} />
              </div>
              <div className="col-sm-12 col-md-4">
                <label className="form-label" htmlFor="gameTimeFromFilter">Game time from</label>
                <input id="gameTimeFromFilter" type="datetime-local" className="form-control form-control-sm" value={timeFrom} onChange={(e) => setTimeFrom(e.target.value)} />
              </div>
              <div className="col-sm-12 col-md-4">
                <label className="form-label" htmlFor="gameTimeToFilter">Game time to</label>
                <input id="gameTimeToFilter" type="datetime-local" className="form-control form-control-sm" value={timeTo} onChange={(e) => setTimeTo(e.target.value)} />
              </div>
            </div>
            <table className="table table-sm">
              <thead>
                <tr className="small table-dark">
                  <th>Created</th><th>Username</th><th>Last activity</th><th>Game</th><th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((g: GameInfo) => {
                  const style = activityColor(g, 20);
                  return (
                    <tr key={g.GameId} className="small table-dark">
                      <td style={style}>{longDate(g.CreationDate)}</td>
                      <td style={style}>{g.CreatorUsername}</td>
                      <td style={style}>{g.LastActivity ? longDate(g.LastActivity, true) : ""}</td>
                      <td style={style}>{g.Name}</td>
                      <td>
                        <button className="btn btn-primary btn-sm me-1" title="Download savegame" aria-label="Download savegame" onClick={async () => log(await client.adminDownloadGame(g.GameId))}>
                          <span className="oi oi-data-transfer-download" aria-hidden="true"></span>
                        </button>{" "}
                        <button className="btn btn-primary btn-sm me-1" onClick={async () => log(await client.adminCloseGame(g.GameId))}>Close</button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
        <div className="card mb-2">
          <div className="card-header">Scheduled games</div>
          <div className="card-body">
            <table className="table table-sm">
              <tbody>
                {scheduled.map((g: ScheduledGameInfo) => (
                  <tr key={g.ScheduledGameId} className="small table-dark">
                    <td className="w-25">{longDate(g.DateTime, true)}</td>
                    <td className="w-25">{g.CreatorName}</td>
                    <td className="w-25"><button className="btn btn-primary btn-sm me-1" onClick={async () => log(await client.adminCancelGame(g.ScheduledGameId))}>Cancel</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
      <div className="col-sm-12 col-lg-2">
        <div className="card mb-2">
          <div className="card-header">Log</div>
          <div className="card-body">{messages.map((m, i) => <div key={i}>{m}</div>)}</div>
        </div>
      </div>
      <div className="col-sm-12 col-lg-2"></div>
    </div>
  );
}

/** Route `/admin`. */
export function AdminPage() {
  return <WithLobby admin><MainLayout><AdminScreen /></MainLayout></WithLobby>;
}
