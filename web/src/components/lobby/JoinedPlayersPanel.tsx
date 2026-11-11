// GenericComponents/JoinedPlayersPanel.razor: who is in the game, with the host's controls
// (make host, kick, open or close a seat), leaving, re-seating, and the observers.
import { useState } from "react";
import { PhaseValue, type Faction } from "../../generated/enums";
import { useLobby } from "../../lobby/context";
import { BotIcon, HostIcon } from "./Icons";
import { CommandButton, FactionImage } from "./widgets";

export function JoinedPlayersPanel() {
  const [client, state] = useLobby();
  const g = state.game!;
  const p = g.participation;
  const view = g.view;
  const phase = view?.phase ?? "AwaitingPlayers";
  const [actionResult, setActionResult] = useState("");
  const me = client.userId;
  const iAmHost = p.isHost(me);

  const setOrUnsetHost = async (userId: number) => setActionResult(await client.requestSetOrUnsetHost(userId));
  const kick = async (userId: number) => setActionResult(await client.requestKick(userId));
  const openSeat = async (seat: number) => setActionResult(await client.requestOpenOrCloseSeat(seat));
  const leave = async () => {
    await client.requestLeaveGame();
    setActionResult("");
  };

  let rows;
  if (phase === "AwaitingPlayers") {
    const players = p.seatedPlayers.keys();
    rows = players.map((userId) => {
      const isMe = me === userId;
      const isHost = p.isHost(userId);
      return (
        <div key={userId} className={userId === players[0] ? "" : "hairline-top"} style={{ verticalAlign: "center", minHeight: "2rem" }}>
          <span className={isMe ? "player-name you" : "player-name"}>{p.playerName(userId)}</span>
          {isHost && <HostIcon size={18} />}
          {iAmHost && (
            <>
              {!isHost && <CommandButton small marginClass="m-1 ms-2" onClick={() => setOrUnsetHost(userId)}>Make host</CommandButton>}
              {!isMe && <CommandButton small className="btn-danger" marginClass="m-1" onClick={() => kick(userId)}>Kick</CommandButton>}
            </>
          )}
        </div>
      );
    });
  } else {
    const players = [...(view?.players ?? [])].sort((a, b) => a.seat - b.seat);
    rows = players.map((player, i) => {
      const userId = p.userIdInSeat(player.seat);
      const isBot = !p.seatedPlayers.values().includes(player.seat);
      const isMe = me === userId;
      const isHost = p.isHost(userId);
      const available = p.seatIsAvailable(player.seat);
      return (
        <div key={player.seat} className={i === 0 ? "" : "hairline-top"} style={{ minHeight: "2rem" }}>
          {player.faction !== "None" && <FactionImage faction={player.faction as Faction} diameter={20} />}
          {isBot ? <BotIcon size={18} /> : <span className={`m-1 ${isMe ? "player-name you" : "player-name"}`}>{p.playerName(userId)}</span>}
          {isHost && <HostIcon size={18} />}
          {available && <span className="badge text-bg-success">open</span>}
          {iAmHost && (
            <>
              <CommandButton small marginClass="m-1" onClick={() => openSeat(player.seat)}>{available ? "Close" : "Open"} seat</CommandButton>
              {!isBot && (
                <>
                  {!isHost && <CommandButton small marginClass="m-1" onClick={() => setOrUnsetHost(userId)}>Make host</CommandButton>}
                  {!isMe && <CommandButton small className="btn-danger" marginClass="m-1" onClick={() => kick(userId)}>Kick</CommandButton>}
                </>
              )}
              <br />
            </>
          )}
        </div>
      );
    });
  }

  const observers = p.observerNames();
  return (
    <div className="mt-2 p-1 border">
      {rows}
      <div className="text-danger align-bottom">{actionResult}</div>
      {(!iAmHost || PhaseValue[phase] > PhaseValue.AwaitingPlayers) && (
        <CommandButton small className="btn-danger" marginClass="mt-2" onClick={leave}>Leave game</CommandButton>
      )}
      {iAmHost && PhaseValue[phase] >= PhaseValue.TradingFactions && (
        <CommandButton small className="btn-warning" marginClass="mt-2 ms-1" onClick={() => client.requestReseat()}>Re-seat players</CommandButton>
      )}
      {p.observers.size > 0 && (
        <>
          <div><label className="mt-1 mb-0">Observers</label></div>
          <div>{observers.map((o, i) => <span key={i} className="me-1 player-name">{o}</span>)}</div>
        </>
      )}
    </div>
  );
}
