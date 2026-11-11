// OtherComponents/RunningGamesComponent.razor: the active games (open ones first, then full ones)
// and the scheduled games.
import { useEffect, useState } from "react";
import type { Faction } from "../../generated/enums";
import { useLobby } from "../../lobby/context";
import { HEARTBEAT_DELAY } from "../../lobby/client";
import { dictGet, type GameInfo, youAreIn, yourCurrentSeat } from "../../lobby/types";
import { BotIcon, PlayIcon, UserIcon } from "./Icons";
import { SubscribeComponent } from "./SubscribeComponent";
import { CommandButton, FactionImage, FactionShown, shortDate } from "./widgets";

/** `StartRedrawCycle(interval)`: redraw every `interval` ms (the colours follow the clock). */
export function useRedrawCycle(interval: number): void {
  const [, setTick] = useState(0);
  useEffect(() => {
    const t = window.setInterval(() => setTick((n) => n + 1), interval);
    return () => window.clearInterval(t);
  }, [interval]);
}

/** Games whose last action is longer ago than `minutes` are drawn in the faint text colour. */
export function activityColor(g: GameInfo, minutes: number): React.CSSProperties {
  const stale = g.LastActivity != null && (Date.now() - Date.parse(g.LastActivity)) / 60_000 > minutes;
  return { color: stale ? "var(--text-3)" : "var(--parchment)" };
}

export function RunningGamesComponent() {
  const [client, state] = useLobby();
  const { lobby } = client;
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  const [joinErrors, setJoinErrors] = useState<Record<string, string>>({});
  useRedrawCycle(HEARTBEAT_DELAY);

  const awaiting = lobby.value("Phase", "AwaitingPlayers");
  const selectingFactions = lobby.value("Phase", "SelectingFactions");
  const faction = (f: number) => lobby.name<Faction>("Faction", f);
  const userId = client.userId;

  const joinAsPlayer = async (gameId: string, password: string, seat: number) => {
    const error = await client.requestJoinGame(gameId, password, seat);
    setJoinErrors((e) => ({ ...e, [gameId]: error ?? "" }));
  };
  const joinAsObserver = async (gameId: string, password: string) => {
    const error = await client.requestObserveGame(gameId, password);
    setJoinErrors((e) => ({ ...e, [gameId]: error ?? "" }));
  };

  const open = state.activeGamesWithOpenSeats;
  const full = state.activeGamesWithoutOpenSeats;

  const factionIcons = (g: GameInfo) => (g.FactionsInPlay ?? []).map((f, i) => <FactionShown key={i} faction={faction(f)} width={18} noShadow />);

  const openRow = (av: GameInfo) => {
    const style = activityColor(av, 15);
    const password = passwords[av.GameId] ?? "";
    return (
      <tr key={av.GameId} className="small table-dark">
        <td style={style}><span style={{ fontSize: "14px" }}>{av.Name}</span></td>
        <td style={style}>
          {av.Phase === awaiting ? (
            <div><UserIcon size={20} title="Players" marginClass="" /><span className="me-1">{av.NrOfPlayers}/{av.MaxPlayers}</span></div>
          ) : (
            <>
              <div style={{ display: "inline-block" }}><UserIcon size={20} title="Players" marginClass="" /><span className="me-1">{av.NrOfPlayers}</span></div>
              {av.NrOfBots > 0 && <div style={{ display: "inline-block" }}><BotIcon size={16} title="Bots" marginClass="me-1" /><span>{av.NrOfBots}</span></div>}
            </>
          )}
        </td>
        <td style={style}>
          {av.Phase === awaiting ? (
            <div>{lobby.label("Ruleset", av.Ruleset)}, {av.MaxTurns} turns, not yet started</div>
          ) : (
            <>
              <div>{lobby.label("Ruleset", av.Ruleset)}, Turn {av.Turn}: {lobby.label("MainPhase", av.MainPhase)}</div>
              <div>{factionIcons(av)}</div>
            </>
          )}
        </td>
        <td>
          {!youAreIn(av, userId) ? (
            <>
              {av.HasPassword && (
                <input className="form-inline form-control-sm" style={{ maxWidth: "80px" }} value={password} maxLength={24} placeholder="Password"
                       onChange={(e) => setPasswords((p) => ({ ...p, [av.GameId]: e.target.value }))} />
              )}
              {av.Phase === awaiting ? (
                <CommandButton small onClick={() => joinAsPlayer(av.GameId, password, -1)} title="Join game" marginClass="m-1">Join</CommandButton>
              ) : av.Phase !== selectingFactions ? (
                av.AvailableSeats.map((s) => (
                  <CommandButton key={s.Seat} small onClick={() => joinAsPlayer(av.GameId, password, s.Seat)} marginClass="m-1" paddingClass="p-1"
                                 title={`Join game as ${lobby.label("Faction", s.Faction)}`}>
                    <FactionImage faction={faction(s.Faction)} diameter={20} />
                  </CommandButton>
                ))
              ) : null}
              <CommandButton small onClick={() => joinAsObserver(av.GameId, password)} title="Observe game" marginClass="m-1">👁</CommandButton>
            </>
          ) : (
            <CommandButton small onClick={() => joinAsPlayer(av.GameId, "", yourCurrentSeat(av, userId))} title="Resume game" marginClass="m-1"><PlayIcon /></CommandButton>
          )}
          <div className="text-danger">{joinErrors[av.GameId] ?? ""}</div>
        </td>
      </tr>
    );
  };

  const fullRow = (av: GameInfo) => {
    const style = activityColor(av, 20);
    const password = passwords[av.GameId] ?? "";
    return (
      <tr key={av.GameId} className="small table-dark">
        <td style={style}><span style={{ fontSize: "14px" }}>{av.Name}</span></td>
        <td style={style}>
          <div style={{ display: "inline-block" }}><UserIcon size={20} title="Players" marginClass="" /><span className="me-1">{av.NrOfPlayers}</span></div>
          {av.NrOfBots > 0 && <div style={{ display: "inline-block" }}><BotIcon size={16} title="Bots" marginClass="me-1" /><span>{av.NrOfBots}</span></div>}
        </td>
        <td style={style}>
          {av.Phase === awaiting ? (
            <div>{lobby.label("Ruleset", av.Ruleset)}, {av.MaxTurns} turns</div>
          ) : (
            <div>
              <div>{lobby.label("Ruleset", av.Ruleset)}, Turn {av.Turn}: {lobby.label("MainPhase", av.MainPhase)}</div>
              <div>{factionIcons(av)}</div>
            </div>
          )}
        </td>
        <td>
          {!youAreIn(av, userId) ? (
            <>
              <span className="full-tag" title="No open seats">Full</span>
              <CommandButton small onClick={() => joinAsObserver(av.GameId, password)} title="Observe game" marginClass="m-1">👁</CommandButton>
            </>
          ) : (
            <CommandButton small onClick={() => joinAsPlayer(av.GameId, "", yourCurrentSeat(av, userId))} title="Resume game" marginClass="m-1"><PlayIcon /></CommandButton>
          )}
          <div className="text-danger">{joinErrors[av.GameId] ?? ""}</div>
        </td>
      </tr>
    );
  };

  return (
    <div className="card mb-2 shadow-lg" style={{ animation: "fadeIn 1s" }}>
      <div className="card-header">
        <div className="d-flex justify-content-between">
          <div>Active games</div>
        </div>
      </div>
      <div className="card-body">
        {open.length === 0 && full.length === 0 && state.scheduledGames.length === 0 ? (
          <div className="text-info">No games...</div>
        ) : (
          <>
            {(open.length > 0 || full.length > 0) && (
              <table className="table table-sm table-running-games">
                <tbody>
                  {open.map(openRow)}
                  {full.map(fullRow)}
                </tbody>
              </table>
            )}
            {state.scheduledGames.length > 0 && (
              <>
                <div className="games-section-label">Scheduled</div>
                <table className="table table-sm table-scheduled-games">
                  <thead>
                    <tr className="small table-dark">
                      <th scope="col">Host</th>
                      <th scope="col">When</th>
                      <th scope="col">Game info</th>
                      <th scope="col">Subscribers</th>
                      <th scope="col">Join?</th>
                    </tr>
                  </thead>
                  <tbody>
                    {state.scheduledGames.map((game) => (
                      <tr key={game.ScheduledGameId} className="small table-dark">
                        <td><span style={{ fontSize: "18px" }}>{game.CreatorName}</span></td>
                        <td>{shortDate(game.DateTime)}</td>
                        <td>
                          <div>
                            {lobby.label("Ruleset", game.Ruleset)}, {game.NumberOfPlayers} players, {game.MaximumTurns} turns
                            {game.AllowedFactionsInPlay.length === 12
                              ? <span> (all factions)</span>
                              : <div>{game.AllowedFactionsInPlay.map((f, i) => <FactionShown key={i} faction={faction(f)} width={18} noShadow />)}</div>}
                          </div>
                        </td>
                        <td>
                          <Subscribers game={game} size={15} />
                        </td>
                        <td><SubscribeComponent scheduledGame={game} /></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

/** The subscribers of a scheduled game who take part, with their subscription. */
export function Subscribers({ game, size }: { game: import("../../lobby/types").ScheduledGameInfo; size: number }) {
  const [client] = useLobby();
  const dontParticipate = client.lobby.value("SubscriptionType", "DontParticipate");
  return (
    <>
      {game.Subscribers.filter(([, s]) => s !== dontParticipate).map(([id, s]) => (
        <div key={id} className="me-1">
          <span style={{ fontSize: `${size}px` }}>{dictGet(game.SubscriberNames, id) ?? "?"}</span>{" "}
          <span className="badge text-bg-primary ms-1">{client.lobby.label("SubscriptionType", s)}</span>
        </div>
      ))}
    </>
  );
}
