// GenericComponents/JoinedPlayersPanel.razor for a game shown without the lobby client (a saved
// view, a scene): every seat in seat order with its faction, its player (or a bot), and "Leave
// game". Inside the lobby, MenuPanels shows the lobby's panel (components/lobby), which has the
// game's participation for the host's controls, the hosts and the observers.
import { useState } from "react";
import { useGame } from "../state/context";
import { FactionComponent } from "./FactionComponent";
import { BotIcon } from "./lobby/Icons";

export function JoinedPlayersPanel() {
  const { view, source } = useGame();
  const [result, setResult] = useState("");
  const players = [...view.players].sort((a, b) => a.seat - b.seat);
  const leave = async () => setResult((await source.gameCall?.("leaveGame", {})) ?? "");
  return (
    <div className="joined-players">
      {players.map((p, i) => (
        <div key={p.seat} className={i === 0 ? "" : "hairline-top"} style={{ minHeight: "2rem" }}>
          {p.faction !== "None" && <FactionComponent faction={p.faction} diameter={20} />}
          {p.isBot
            ? <BotIcon size={18} />
            : <span className={`joined-players__name ${view.viewer.faction === p.faction && !view.viewer.isObserver ? "player-name player-name--you" : "player-name"}`}>{p.name}</span>}
        </div>
      ))}
      <div className="joined-players__result">{result}</div>
      <button className="btn btn-danger btn-sm joined-players__leave" onClick={leave}>Leave game</button>
    </div>
  );
}
