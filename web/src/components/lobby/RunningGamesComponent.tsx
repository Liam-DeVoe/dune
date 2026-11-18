// OtherComponents/RunningGamesComponent.razor: the active games (open ones first, then full ones)
// and the scheduled games.
import { useEffect, useState } from "react";
import { useLobby } from "../../lobby/context";
import { HEARTBEAT_DELAY } from "../../lobby/client";
import { type GameInfo, type ScheduledGameInfo, youAreIn, yourCurrentSeat } from "../../lobby/types";
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
  const stale = g.lastActivity != null && (Date.now() - Date.parse(g.lastActivity)) / 60_000 > minutes;
  return { color: stale ? "var(--text-3)" : "var(--parchment)" };
}

export function RunningGamesComponent() {
  const [client, state] = useLobby();
  const { lobby } = client;
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  const [joinErrors, setJoinErrors] = useState<Record<string, string>>({});
  useRedrawCycle(HEARTBEAT_DELAY);

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

  const factionIcons = (g: GameInfo) => g.factionsInPlay.map((f, i) => <FactionShown key={i} faction={f} width={18} noShadow />);

  const openRow = (av: GameInfo) => {
    const style = activityColor(av, 15);
    const password = passwords[av.gameId] ?? "";
    return (
      <tr key={av.gameId} className="games-table__row">
        <td style={style}><span style={{ fontSize: "14px" }}>{av.name}</span></td>
        <td style={style}>
          {av.phase === "AwaitingPlayers" ? (
            <div><UserIcon size={20} title="Players" marginClass="" /><span className="games-table__count">{av.players}/{av.maxPlayers}</span></div>
          ) : (
            <>
              <div style={{ display: "inline-block" }}><UserIcon size={20} title="Players" marginClass="" /><span className="games-table__count">{av.players}</span></div>
              {av.bots > 0 && <div style={{ display: "inline-block" }}><BotIcon size={16} title="Bots" marginClass="games-table__bots-icon" /><span>{av.bots}</span></div>}
            </>
          )}
        </td>
        <td style={style}>
          {av.phase === "AwaitingPlayers" ? (
            <div>{lobby.labelOf("Ruleset", av.ruleset)}, {av.maxTurns} turns, not yet started</div>
          ) : (
            <>
              <div>{lobby.labelOf("Ruleset", av.ruleset)}, Turn {av.turn}: {lobby.labelOf("MainPhase", av.mainPhase)}</div>
              <div>{factionIcons(av)}</div>
            </>
          )}
        </td>
        <td>
          {!youAreIn(av, userId) ? (
            <>
              {av.hasPassword && (
                <input className="input input--small" style={{ maxWidth: "80px" }} value={password} maxLength={24} placeholder="Password"
                       onChange={(e) => setPasswords((p) => ({ ...p, [av.gameId]: e.target.value }))} />
              )}
              {av.phase === "AwaitingPlayers" ? (
                <CommandButton small onClick={() => joinAsPlayer(av.gameId, password, -1)} title="Join game" marginClass="games-table__action">Join</CommandButton>
              ) : av.phase !== "SelectingFactions" ? (
                av.availableSeats.map((s) => (
                  <CommandButton key={s.seat} small onClick={() => joinAsPlayer(av.gameId, password, s.seat)} marginClass="games-table__action" paddingClass="games-table__seat"
                                 title={`Join game as ${lobby.labelOf("Faction", s.faction)}`}>
                    <FactionImage faction={s.faction} diameter={20} />
                  </CommandButton>
                ))
              ) : null}
              <CommandButton small onClick={() => joinAsObserver(av.gameId, password)} title="Observe game" marginClass="games-table__action">👁</CommandButton>
            </>
          ) : (
            <CommandButton small onClick={() => joinAsPlayer(av.gameId, "", yourCurrentSeat(av, userId))} title="Resume game" marginClass="games-table__action"><PlayIcon /></CommandButton>
          )}
          <div className="games-table__error">{joinErrors[av.gameId] ?? ""}</div>
        </td>
      </tr>
    );
  };

  const fullRow = (av: GameInfo) => {
    const style = activityColor(av, 20);
    const password = passwords[av.gameId] ?? "";
    return (
      <tr key={av.gameId} className="games-table__row">
        <td style={style}><span style={{ fontSize: "14px" }}>{av.name}</span></td>
        <td style={style}>
          <div style={{ display: "inline-block" }}><UserIcon size={20} title="Players" marginClass="" /><span className="games-table__count">{av.players}</span></div>
          {av.bots > 0 && <div style={{ display: "inline-block" }}><BotIcon size={16} title="Bots" marginClass="games-table__bots-icon" /><span>{av.bots}</span></div>}
        </td>
        <td style={style}>
          {av.phase === "AwaitingPlayers" ? (
            <div>{lobby.labelOf("Ruleset", av.ruleset)}, {av.maxTurns} turns</div>
          ) : (
            <div>
              <div>{lobby.labelOf("Ruleset", av.ruleset)}, Turn {av.turn}: {lobby.labelOf("MainPhase", av.mainPhase)}</div>
              <div>{factionIcons(av)}</div>
            </div>
          )}
        </td>
        <td>
          {!youAreIn(av, userId) ? (
            <>
              <span className="full-tag" title="No open seats">Full</span>
              <CommandButton small onClick={() => joinAsObserver(av.gameId, password)} title="Observe game" marginClass="games-table__action">👁</CommandButton>
            </>
          ) : (
            <CommandButton small onClick={() => joinAsPlayer(av.gameId, "", yourCurrentSeat(av, userId))} title="Resume game" marginClass="games-table__action"><PlayIcon /></CommandButton>
          )}
          <div className="games-table__error">{joinErrors[av.gameId] ?? ""}</div>
        </td>
      </tr>
    );
  };

  return (
    <div className="card running-games" style={{ animation: "fadeIn 1s" }}>
      <div className="card__header">
        <div className="running-games__header">
          <div>Active games</div>
        </div>
      </div>
      <div className="card__body">
        {open.length === 0 && full.length === 0 && state.scheduledGames.length === 0 ? (
          <div className="running-games__empty">No games...</div>
        ) : (
          <>
            {(open.length > 0 || full.length > 0) && (
              <table className="table table--small table--dark games-table games-table--running">
                <tbody>
                  {open.map(openRow)}
                  {full.map(fullRow)}
                </tbody>
              </table>
            )}
            {state.scheduledGames.length > 0 && (
              <>
                <div className="games-section-label">Scheduled</div>
                <table className="table table--small table--dark games-table games-table--scheduled">
                  <thead>
                    <tr className="games-table__row">
                      <th scope="col">Host</th>
                      <th scope="col">When</th>
                      <th scope="col">Game info</th>
                      <th scope="col">Subscribers</th>
                      <th scope="col">Join?</th>
                    </tr>
                  </thead>
                  <tbody>
                    {state.scheduledGames.map((game) => (
                      <tr key={game.scheduledGameId} className="games-table__row">
                        <td><span style={{ fontSize: "18px" }}>{game.creatorName}</span></td>
                        <td>{shortDate(game.starts)}</td>
                        <td>
                          <div>
                            {lobby.labelOf("Ruleset", game.ruleset)}, {game.players} players, {game.maxTurns} turns
                            {game.allowedFactions.length === 12
                              ? <span> (all factions)</span>
                              : <div>{game.allowedFactions.map((f, i) => <FactionShown key={i} faction={f} width={18} noShadow />)}</div>}
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
export function Subscribers({ game, size }: { game: ScheduledGameInfo; size: number }) {
  const [client] = useLobby();
  return (
    <>
      {game.subscribers.filter((s) => s.subscription !== "DontParticipate").map((s) => (
        <div key={s.userId} className="subscribers__entry">
          <span style={{ fontSize: `${size}px` }}>{s.userName}</span>{" "}
          <span className="badge badge--primary subscribers__badge">{client.lobby.labelOf("SubscriptionType", s.subscription)}</span>
        </div>
      ))}
    </>
  );
}
