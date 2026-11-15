// Routes. /scene/:fixture/:faction is one game in a fixed state,
// seen by one faction. With the Rust server running with DevScene on, it opens that game on the
// server (POST /dev/scene) and plays it over the WebSocket; without one, it renders the saved view
// (web/public/views, written by rust/view/examples/scene_views.rs). /game/:gameId reconnects to a
// game on the server with the session the browser keeps, and /link/:linkToken logs in with the
// single-use link of an async-play mail. Session tokens never appear in URLs. scene.ts gives
// Playwright scripts hooks into the scene page.
import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Route, Routes, useParams } from "react-router";
import { GameScreen } from "./components/GameScreen";
import { MainLayout } from "./components/MainLayout";
import { Game, type GameContext } from "./state/context";
import { Connection, hashPassword, login, storeSession } from "./state/connection";
import { StaticSource, type ViewSource, WebSocketSource } from "./state/source";
import { loadSkin } from "./skin";
import { sceneFailed, sceneReady } from "./scene";
import { MainPage } from "./components/lobby/MainPage";
import { AdminPage } from "./components/lobby/AdminPage";
import { BASE, withBase } from "./base";

/** The scene game on the server, or null when there is no server with DevScene on. */
async function sceneOnServer(fixture: string, faction: string): Promise<WebSocketSource | null> {
  let r: Response;
  try {
    r = await fetch(withBase("dev/scene"), { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ fixture, faction }) });
  } catch {
    return null;
  }
  if (r.status === 404 || r.status === 405 || r.status >= 500) return null;
  if (!r.ok) throw new Error(`/dev/scene: ${await r.text()}`);
  const { userToken, gameId } = (await r.json()) as { userToken: string; gameId: string };
  const conn = new Connection();
  conn.setSession(userToken);
  return new WebSocketSource(conn, gameId);
}

/** Loads a source and the skin, then follows the source's later views. */
function useGameFrom(open: () => Promise<ViewSource>, deps: unknown[]): { game: GameContext | null; error: string | null } {
  const [game, setGame] = useState<GameContext | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    let unsubscribe: (() => void) | undefined;
    let source: ViewSource | undefined;
    open()
      .then((s) => { source = s; return Promise.all([s.load(), loadSkin()]); })
      .then(([view, skin]) => {
        if (!live || !source) return;
        setGame({ view, skin, source });
        unsubscribe = source.subscribe?.((v) => setGame((g) => (g ? { ...g, view: v } : g)));
      })
      .catch((e: unknown) => { if (live) setError(String(e)); });
    return () => {
      live = false;
      unsubscribe?.();
      if (source instanceof WebSocketSource) source.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return { game, error };
}

function Scene() {
  const { fixture, faction } = useParams();
  const { game, error } = useGameFrom(
    async () => (await sceneOnServer(fixture!, faction!)) ?? new StaticSource(withBase(`views/${fixture}-${faction}.json`)),
    [fixture, faction],
  );
  // Effects run after the commit, so the scene is on screen when the harness hears "ready".
  useEffect(() => { if (game) sceneReady(); }, [game]);
  useEffect(() => { if (error) sceneFailed(error); }, [error]);
  if (error) return <div className="alert alert-danger">{error}</div>;
  if (!game) return null;
  return <MainLayout><Game.Provider value={game}><GameScreen /></Game.Provider></MainLayout>;
}

// The connection code, for scripts that set up games before opening them (the end-to-end test
// scripts/ui/e2e_server.py logs in, creates and seats a game with it, and keeps the session with
// storeSession so that /game/:gameId opens the game logged in).
declare global {
  interface Window { treacheryClient?: { Connection: typeof Connection; login: typeof login; hashPassword: typeof hashPassword; storeSession: typeof storeSession } }
}
window.treacheryClient = { Connection, login, hashPassword, storeSession };

function Home() {
  return (
    <div className="p-4">
      <p>treachery.online web client skeleton. Saved scenes:</p>
      <ul>
        {["alliance/Emperor", "alliance/Atreides", "alliance-allied/BeneGesserit"].map((s) => <li key={s}><a href={withBase(`scene/${s}`)}>{s}</a></li>)}
      </ul>
    </div>
  );
}

createRoot(document.getElementById("app")!).render(
  <StrictMode>
    <BrowserRouter basename={BASE}>
      <Routes>
        {/* The lobby, rejoining a game, and the login link of an async-play mail. */}
        <Route path="/" element={<MainPage />} />
        <Route path="/admin" element={<AdminPage />} />
        <Route path="/scenes" element={<Home />} />
        <Route path="/scene/:fixture/:faction" element={<Scene />} />
        <Route path="/game/:gameId" element={<MainPage />} />
        <Route path="/link/:linkToken" element={<MainPage />} />
      </Routes>
    </BrowserRouter>
  </StrictMode>,
);
