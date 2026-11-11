// OtherComponents/MenuPanels.razor: what the menu bar under the map (MenuComponent) opens, shown
// at the top of the side column: the players list, the automation rules, the host's undo panel,
// and errors from loading a game or skin.
import { useContext } from "react";
import { LobbyContext } from "../lobby/context";
import { useGame } from "../state/context";
import { JoinedPlayersPanel as LobbyPlayersPanel } from "./lobby/JoinedPlayersPanel";
import { JoinedPlayersPanel } from "./JoinedPlayersPanel";
import { ActionPanel } from "./ActionPanel";
import { menu, useMenu } from "./menuState";
import { UndoComponent } from "./UndoComponent";

export function MenuPanels() {
  const { view } = useGame();
  const m = useMenu();
  // The lobby keeps the game's participation (hosts, open seats, observers) when there is one.
  const lobby = useContext(LobbyContext);
  return (
    <>
      {m.loadError !== "" && (
        <>
          <div className="text-danger small">{m.loadError}</div>
          <button className="btn btn-primary mt-1 mb-2 btn-sm" onClick={() => menu.setLoadError("")}>Clear errors</button>
        </>
      )}
      {m.panel === "Undo"
        ? <UndoComponent onClose={menu.close} />
        : m.panel === "Players"
          ? (lobby?.getState().game ? <LobbyPlayersPanel /> : <JoinedPlayersPanel />)
          : m.panel === "Automation" && view.actions.some((a) => a.eventType === "AutomationConfigured")
            && <ActionPanel filter={(a) => a.eventType === "AutomationConfigured"} />}
    </>
  );
}
