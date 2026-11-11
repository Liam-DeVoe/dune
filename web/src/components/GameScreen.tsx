// OtherComponents/GameColumns.razor: the board column, then one column with your faction, the
// prompt and actions, and the log.
//
// While the host previews an undo, everything shows the game at the chosen moment
// (Client.ShownGame): the screen gets the preview as its view. The provider is always there, so
// switching to and from a preview never remounts the panels (the undo panel would close).
import { useMemo } from "react";
import { Game, useGame } from "../state/context";
import { ActionPanel } from "./ActionPanel";
import { CardPlayPopover } from "./CardPlayPopover";
import { GameBoard } from "./board/GameBoard";
import { ImageFlash } from "./ImageFlash";
import { LogPanel } from "./LogPanel";
import { PlayerPanel } from "./PlayerPanel";
import { GameStatusComponent } from "./GameStatusComponent";
import { MenuComponent } from "./MenuComponent";
import { MenuPanels } from "./MenuPanels";
import { TurnBanner, turnBannerStyle } from "./TurnBanner";
import { UndoTimeline } from "./UndoTimeline";
import { useUndoSession } from "./undoSession";
import { useMilestoneSounds } from "./milestoneSounds";
import { PhaseValue } from "../generated/enums";
import type { GameView } from "../generated/GameView";

export function GameScreen() {
  const game = useGame();
  useMilestoneSounds(game.view, game.skin);
  const preview = useUndoSession()?.preview ?? null;
  const shown = useMemo(() => (preview ? { ...game, view: preview } : game), [game, preview]);
  return <Game.Provider value={shown}><Screen live={game.view} /></Game.Provider>;
}

function Screen({ live }: { live: GameView }) {
  const { view } = useGame();
  return (
    <>
    <TurnBanner />
    <div className="row game-screen" style={turnBannerStyle(view)}>
      <div id="mapdiv" className="col-sm-12">
        <div className="board-area">
          <GameBoard />
          <MenuComponent />
        </div>
        <UndoTimeline />
      </div>
      <div className="col-sm-12 cursor-hand side-column">
        <MenuPanels />
        {!view.viewer.isObserver && PhaseValue[view.phase] > PhaseValue.TradingFactions && <PlayerPanel />}
        <div className="act-stack">
          <GameStatusComponent />
          <ActionPanel />
        </div>
        <LogPanel />
      </div>
      <CardPlayPopover />
      <ImageFlash view={live} />
    </div>
    </>
  );
}
