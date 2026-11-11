// OtherComponents/SeatPlayersComponent.razor: the host assigns each player a seat, by faction once
// factions are known, by seat number before that.
import { useState } from "react";
import type { Faction } from "../../generated/enums";
import { useLobby } from "../../lobby/context";
import { NetDict } from "../../lobby/participation";
import { FactionImage, SelectFaction } from "./widgets";

export function SeatPlayersComponent() {
  const [client, state] = useLobby();
  const g = state.game!;
  const players = g.view?.players ?? [];
  // OnParametersSet: a copy of the current seats.
  const [seated, setSeated] = useState(() => g.participation.seatedPlayers.clone());

  const assignSeat = (userId: number, seat: number) => {
    const next = seated.clone();
    for (const [other, s] of next.items()) if (other !== userId && s === seat) next.set(other, -1);
    next.set(userId, seat);
    setSeated(next);
  };
  const assignFaction = (userId: number, faction: Faction | null) => {
    assignSeat(userId, players.find((p) => p.faction === faction)?.seat ?? -1);
  };
  const assignedFaction = (userId: number): Faction | null => {
    const seat = seated.get(userId) ?? 0;
    if (seat === -1) return null;
    const f = players.find((p) => p.seat === seat)?.faction as Faction | undefined;
    return f && f !== "None" ? f : null;
  };
  const anyWithoutFaction = players.some((p) => p.faction === "None");

  return (
    <div className="card mb-2 shadow-lg" style={{ animation: "fadeInLate 3s" }}>
      <div className="card-header">Assign seats to players</div>
      <div className="card-body">
        <table className="table table-sm">
          <thead>
            <tr className="table-dark">
              <th scope="col">Player</th>
              <th scope="col">Seat</th>
            </tr>
          </thead>
          <tbody>
            {seated.keys().map((userId) => {
              const current = seated.get(userId) ?? Number.MIN_SAFE_INTEGER;
              return (
                <tr key={userId} className="table-dark">
                  <td>{g.participation.playerName(userId)}</td>
                  <td>
                    {anyWithoutFaction
                      ? players.map((p) => {
                        const size = p.seat === current ? 60 : 30;
                        return p.faction !== "None"
                          ? <FactionImage key={p.seat} diameter={size} faction={p.faction as Faction} onClick={() => assignFaction(userId, p.faction as Faction)} />
                          : <SeatNumber key={p.seat} size={size} value={p.seat} onClick={() => assignSeat(userId, p.seat)} />;
                      })
                      : <SelectFaction values={players.map((p) => p.faction as Faction)} value={assignedFaction(userId)} required={false} imageWidth={40}
                                       onChange={(f) => assignFaction(userId, f)} />}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <button className="btn btn-primary mt-1" disabled={seated.values().includes(-1)} onClick={() => client.requestAssignSeats((seated as NetDict<number>).toJson())}>Confirm</button>
      </div>
    </div>
  );
}

/** GenericComponents/NumberComponent.razor with a white number background, as the seat picker draws a seat. */
function SeatNumber({ size, value, onClick }: { size: number; value: number; onClick: () => void }) {
  const px = (x: number) => `${x}px`;
  return (
    <div style={{ display: "inline-block", position: "relative", height: px(size), width: px(size), cursor: "pointer" }} title="" onClick={onClick}>
      <svg style={{ position: "absolute", left: 0, top: 0, width: px(size), height: px(size) }} width="16" height="16" viewBox="0 0 16 16">
        <circle cx="8" cy="8" r="7.5" strokeWidth="2%" style={{ stroke: "var(--ink-0)", fill: "var(--parchment)" }} />
      </svg>
      <span style={{ position: "absolute", left: 0, top: px(0.05 * size), width: px(size), fontSize: px(0.7 * size), fontFamily: "var(--font-display)", fontWeight: 400, color: "var(--ink-0)", lineHeight: 1.0, textAlign: "center", whiteSpace: "nowrap" }}>{value}</span>
    </div>
  );
}
