// MapComponents/GameBoard.razor: the map and its layers, in the razor file's order, scaled to
// the board's area by setPlanetMapScale. Lives in GameColumns' .board-area.
import { useCallback, useEffect, useLayoutEffect, useMemo, useSyncExternalStore } from "react";
import { PhaseValue } from "../../generated/enums";
import { useGame } from "../../state/context";
import { BattlePlans } from "./BattlePlans";
import { Board, BoardContext, f, px, useBoard } from "./geometry";
import { MapSelectionHighlights, MapSelectionOverlay, StagedForces } from "./MapSelection";
import { listenForMapRefs, listenForResize, setPlanetMapScale, startMapHover } from "./mapScripts";
import { CardPiles, Phases } from "./Phases";
import { ForcesOnPlanet, Monsters, ResourcesOnPlanet, Ssw } from "./Pieces";
import { MapHover, Planet, RecentBlows } from "./Planet";
import { Bids, Players } from "./Players";
import { RecentMoves } from "./RecentMoves";
import { MapText } from "./primitives";
import { Tanks } from "./Tanks";
import { locationClick } from "./mapInteraction";
import { registerSceneBoard } from "../../scene";

// MapRefArrows.razor: empty; mapScripts.showMapRefs fills it while a log line is hovered.
function MapRefArrows() {
  const b = useBoard();
  const { X: w, Y: h } = b.point("MapDimensions");
  return (
    <svg className="maprefs" width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%" }} pointerEvents="none">
      <defs>
        <marker id="mapref-head" viewBox="0 0 12 12" refX="9" refY="6" markerWidth="4" markerHeight="4" orient="auto-start-reverse" markerUnits="strokeWidth" overflow="visible">
          <path d="M0,0 L12,6 L0,12 Z" className="head" />
        </marker>
      </defs>
      <g className="lines" />
    </svg>
  );
}

export function GameBoard() {
  const { view, skin } = useGame();
  const b = useMemo(() => new Board(view, skin), [view, skin]);
  const { X: w, Y: h } = b.point("MapDimensions");
  const cy = b.center.Y;
  const reach = b.mapRadius + 3 * b.tokenRadius;
  // The crop's top and bottom in map units, which setPlanetMapScale reads.
  const viewTop = cy - reach - 3;
  const viewBottom = cy + reach + 1;

  useLayoutEffect(() => {
    setPlanetMapScale();
    startMapHover();
  });
  useEffect(() => {
    listenForResize();
    listenForMapRefs();
  }, []);
  // The scene page's hooks: SceneScript.LocationCentres, and clicks as Scene.razor makes them.
  useEffect(() => {
    const locations: Record<string, { id: number; x: number; y: number }> = {};
    for (const l of b.planetLocations()) {
      if (!b.hasCenter(l.id)) continue;
      const name = skin.location(l.id);
      if (Object.keys(locations).some((k) => k.toLowerCase() === name.toLowerCase())) continue;
      const c = b.getCenter(l.id);
      locations[name] = { id: l.id, x: Math.round(c.X * 10) / 10, y: Math.round(c.Y * 10) / 10 };
    }
    registerSceneBoard(locations, (id, o) => locationClick({ location: id, shift: !!o.shift, ctrl: !!o.ctrl, alt: false, right: !!o.right }));
  }, [b, skin]);

  return (
    <BoardContext.Provider value={b}>
      <div id="planetmap" style={{ position: "absolute", left: "0px" }}>
        <div id="planetmap_inner" style={{ position: "absolute", width: px(w), height: px(h) }} data-view-top={viewTop} data-view-bottom={viewBottom}>
          <BackgroundImage width={w} height={h} src={b.plainUrl("Map_URL")} />
          {PhaseValue[view.phase] > PhaseValue.AwaitingPlayers && (
            <div style={{ width: "100%", height: "100%" }}>
              <Ssw />
              <RecentBlows />
              <ResourcesOnPlanet />
              <Monsters />
              <div className="map-corner tr"><Phases /></div>
              <div className="map-corner bl"><CardPiles which="spice" /></div>
              <div className="map-corner br"><CardPiles which="treachery" /></div>
              <Planet />
              <MapHover />
              <MapSelectionHighlights />
              <RecentMoves />
              <ForcesOnPlanet />
              <StagedForces />
              <Players />
              <Bids />
              <div className="map-corner tl"><Tanks /></div>
              <Disconnection />
              <BattlePlans />
              <MapRefArrows />
              <MapSelectionOverlay />
            </div>
          )}
        </div>
      </div>
      <div id="planetmap-filler" style={{ display: "block" }} />
    </BoardContext.Provider>
  );
}

// Disconnection.razor: while the connection to the server is down (Client.IsConnected).
function Disconnection() {
  const b = useBoard();
  const { source } = useGame();
  const subscribe = useCallback((onChange: () => void) => source.onConnectionStatus?.(onChange) ?? (() => {}), [source]);
  const status = useSyncExternalStore(subscribe, () => source.connectionStatus?.() ?? "connected");
  if (status === "connected") return null;
  const c = b.center;
  return (
    <>
      <MapText toShow="You were disconnected from the game." x={c.X} y={c.Y} font={b.s("TURN_FONT")} fontColor="darkred"
        borderWidth={b.n("TURN_FONT_BORDERWIDTH")} borderColor="white" align="center" />
      <MapText toShow="Please wait while the game is trying to reconnect..." x={c.X} y={f(1.1) * c.Y} font={b.s("PLAYERNAME_FONT")} fontColor="darkred"
        borderWidth={b.n("PLAYERNAME_FONT_BORDERWIDTH")} borderColor="white" align="center" />
    </>
  );
}

// BackgroundImage.razor: the map as an <object> when it is an SVG, else an <img>.
function BackgroundImage({ width, height, src }: { width: number; height: number; src: string }) {
  const style = { position: "absolute", left: 0, top: 0, width: px(width), height: px(height), pointerEvents: "none" } as const;
  if (src.length >= 3 && src.slice(-3).toUpperCase() === "SVG") {
    return <object style={{ ...style, colorScheme: "light only" }} type="image/svg+xml" data={src} />;
  }
  if (!src) return null;
  return <img style={style} src={src} alt="Map" />;
}
