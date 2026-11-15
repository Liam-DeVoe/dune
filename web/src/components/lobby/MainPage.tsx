// Pages/Main.razor: the start screen. Not connected, then login, then the lobby, then the game the
// user is in: its setup (EstablishPlayers for the host, AwaitingGame for the others), seating,
// and the game screen.
import { useEffect, useState, type ReactNode } from "react";
import { useParams } from "react-router";
import { Game, type GameContext } from "../../state/context";
import { lobbyClient, type LobbyClient } from "../../lobby/client";
import { LobbyContext, useLobby } from "../../lobby/context";
import { GameScreen } from "../GameScreen";
import { MainLayout } from "../MainLayout";
import { AwaitingGameComponent } from "./AwaitingGameComponent";
import { EstablishPlayersComponent } from "./EstablishPlayersComponent";
import { LoginComponent } from "./LoginComponent";
import { OwnGamesComponent } from "./OwnGamesComponent";
import { RunningGamesComponent } from "./RunningGamesComponent";
import { SeatPlayersComponent } from "./SeatPlayersComponent";

export const START_SCREEN_STYLE: React.CSSProperties = {
  minHeight: "97vh", backgroundImage: "url('art/planet.png')", backgroundSize: "1024px", backgroundPositionX: "center",
  backgroundPositionY: "-50px", backgroundRepeat: "no-repeat",
};

/** Starts the page's lobby client (`Client.Start`), then renders `children` with it. */
export function WithLobby({ linkToken, gameId, admin = false, children }: { linkToken?: string; gameId?: string; admin?: boolean; children: ReactNode }) {
  const [client, setClient] = useState<LobbyClient | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    lobbyClient()
      .then(async (c) => {
        if (admin) c.isAdmin = true;
        if (live) setClient(c);
        await c.start({ gameId, linkToken });
      })
      .catch((e: unknown) => { if (live) setError(String(e)); });
    return () => { live = false; };
  }, [linkToken, gameId, admin]);
  if (error) return <MainLayout><div className="alert alert-danger">{error}</div></MainLayout>;
  if (!client) return null;
  return <LobbyContext.Provider value={client}>{children}</LobbyContext.Provider>;
}

/** The centred single column of the login, setup and seating screens. */
function Centered({ children }: { children: ReactNode }) {
  return (
    <div className="row start-screen d-flex align-items-center" style={START_SCREEN_STYLE}>
      <div className="col-sm-12 col-lg-3"></div>
      <div className="col-sm-12 col-lg-6">
        <div className="d-flex justify-content-center">{children}</div>
      </div>
      <div className="col-sm-12 col-lg-3"></div>
    </div>
  );
}

export function MaintenanceBanner() {
  const [client, state] = useLobby();
  if (!client.inScheduledMaintenance() || !state.serverInfo) return null;
  const start = new Date(state.serverInfo.ScheduledMaintenance);
  const end = new Date(start.getTime() + 15 * 60_000);
  return (
    <div className="alert alert-danger text-center p-0" role="alert">
      treachery.online will be down for maintenance from {start.toLocaleString()} until {end.toLocaleString()} (your local time)
    </div>
  );
}

function MainScreen() {
  const [client, state] = useLobby();
  const g = state.game;
  let screen: ReactNode;
  if (!state.connected) {
    screen = (
      <div className="row start-screen d-flex align-items-center" style={START_SCREEN_STYLE}>
        <div className="d-flex justify-content-center">Not Connected</div>
      </div>
    );
  } else if (!state.loginInfo) {
    screen = <Centered><LoginComponent /></Centered>;
  } else if (!g) {
    screen = (
      <div className="row start-screen" style={START_SCREEN_STYLE}>
        <div className="col-sm-12 col-lg-2"></div>
        <div className="col-sm-12 col-lg-5"><RunningGamesComponent /></div>
        <div className="col-sm-12 col-lg-3"><OwnGamesComponent /></div>
        <div className="col-sm-12 col-lg-2"></div>
      </div>
    );
  } else if (!g.view) {
    // Between joining and the first view.
    screen = null;
  } else if (g.view.phase === "AwaitingPlayers") {
    screen = <Centered>{client.isHost ? <EstablishPlayersComponent /> : <AwaitingGameComponent />}</Centered>;
  } else if (client.playersNeedSeating()) {
    screen = <Centered><SeatPlayersComponent key={g.gameId} /></Centered>;
  } else {
    const ctx: GameContext = { view: g.view, skin: client.skin, source: g.source };
    screen = <Game.Provider value={ctx}><GameScreen /></Game.Provider>;
  }
  return (
    <MainLayout>
      <MaintenanceBanner />
      {screen}
    </MainLayout>
  );
}

/** Route `/`; `/game/:gameId` to rejoin a game with the session this browser keeps; and
 * `/link/:linkToken`, the single-use login link of an async-play mail. */
export function MainPage() {
  const { linkToken, gameId } = useParams();
  return <WithLobby linkToken={linkToken} gameId={gameId}><MainScreen /></WithLobby>;
}
