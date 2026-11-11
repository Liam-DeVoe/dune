// OtherComponents/MenuComponent.razor: the bar under the map. Players, automation and a menu; for
// a seated player the trade icons (deals, the alliance, the card tracker). The menu holds
// everything else, leaving the game included, and opens upward. Rules and sound expand in place;
// the host's tools sit in their own section at the bottom. A click outside or Escape closes it.
// The panels the toolbar opens show at the top of the side column (MenuPanels) or in a popover
// above the bar (TradePopover). Styles in _menu.scss and _toolbar.scss.
//
// Differences from C#: Save game asks the server for the live game (RequestSaveGame), where C#
// saved its own copy.
import { useContext, useEffect, useState } from "react";
import { PhaseValue } from "../generated/enums";
import type { GameView } from "../generated/GameView";
import { LobbyContext } from "../lobby/context";
import { useGame } from "../state/context";
import { CreditComponent } from "./CreditComponent";
import { MentatComponent } from "./MentatComponent";
import { MusicComponent } from "./MusicComponent";
import { DealToasts } from "./DealToasts";
import { useMenu, type MenuPanel } from "./menuState";
import { hasCardTools, TradePopover } from "./TradePopover";
import { BASE } from "../base";

// Started: the game has events.
const started = (view: GameView) => view.eventNumber > 0;

// TradeIcons: deals, offering one and giving spice open from here, like the alliance.
export const tradeIcons = (view: GameView) =>
  started(view) && !view.viewer.isObserver && view.me !== null && PhaseValue[view.phase] > PhaseValue.TradingFactions;

export function MenuComponent() {
  const { view, skin, source } = useGame();
  const menu = useMenu();
  const [menuOpen, setMenuOpen] = useState(false);
  const [leaveOpen, setLeaveOpen] = useState(false);
  const [rulesOpen, setRulesOpen] = useState(false);
  const [soundOpen, setSoundOpen] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);

  const has = (t: string) => view.actions.some((a) => a.eventType === t);
  const canAutomate = has("AutomationConfigured");
  const hostTools = started(view) && view.viewer.isHost && !view.viewer.isObserver;
  // CanSave: hosts, and the server's admin (known from the lobby, when there is one).
  const lobby = useContext(LobbyContext)?.getState();
  const isAdmin = lobby?.serverInfo != null && lobby.loginInfo?.UserName === lobby.serverInfo.AdminName;
  const canSave = started(view) && (view.viewer.isHost || isAdmin);
  const numberOfBots = view.players.filter((p) => p.isBot).length;
  const botsPaused = hostTools && numberOfBots > 0 && view.botsSpeed === 0;
  const dealsOffered = view.deals?.acceptable.length ?? 0;
  const ally = view.players.find((p) => p.faction === view.me?.faction)?.ally ?? "None";

  const closeMenus = () => { setMenuOpen(false); setLeaveOpen(false); };
  const togglePanel = (panel: MenuPanel) => { closeMenus(); menu.toggle(panel); };

  // Escape closes the menu (Browser.ListenForEscape).
  useEffect(() => {
    if (!menuOpen && !leaveOpen) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") closeMenus(); };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [menuOpen, leaveOpen]);

  const toggleMenu = () => {
    const open = !menuOpen;
    closeMenus();
    if (open) {
      setMenuOpen(true);
      setFullscreen(document.fullscreenElement !== null);
    }
  };
  const toggleLeave = () => {
    const open = !leaveOpen;
    closeMenus();
    if (open) setLeaveOpen(true);
  };
  const toggleFullScreen = () => {
    closeMenus();
    if (document.fullscreenElement) void document.exitFullscreen();
    else void document.documentElement.requestFullscreen?.();
  };
  // Save: the live game, also while the undo panel previews a moment, as savegame.json
  // (Browser.Save, saveFile in the C# index.html).
  const save = async () => {
    closeMenus();
    if (!source.saveGame) return;
    try {
      const data = await source.saveGame();
      const url = URL.createObjectURL(new Blob([data], { type: "application/json" }));
      const link = document.createElement("a");
      link.download = "savegame.json";
      link.href = url;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    } catch (err) {
      menu.setLoadError(`Error saving game (${String(err)})`);
    }
  };
  // LoadGame and LoadSkin: the file's text to the server, refused when it is too big.
  const loadFile = (method: string, max: number, what: string, args: (data: string) => unknown[]) =>
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      closeMenus();
      const input = e.currentTarget;
      const file = input.files?.[0];
      if (!file) return;
      menu.setLoadError("");
      try {
        const data = await file.text();
        input.value = "";
        if (data.length >= max) {
          menu.setLoadError(`${what} file is too big...`);
          return;
        }
        const error = await source.gameCall?.(method, ...args(data));
        if (error) menu.setLoadError(`Error loading ${what === "Saved Game" ? "game" : "skin"} (${error})`);
      } catch (err) {
        menu.setLoadError(`Error loading ${what === "Saved Game" ? "game" : "skin"} (${String(err)})`);
      }
    };
  const loadGame = loadFile("RequestLoadGame", 4194304, "Saved Game", (data) => [data, null]);
  const loadSkin = loadFile("RequestSetSkin", 262144, "Skin", (data) => [data]);
  const openUndo = () => { closeMenus(); menu.open("Undo"); };
  const setBotSpeed = (speed: number) => { void source.gameCall?.("RequestSetBotSpeed", speed); };

  return (
    <>
    <div className="menu-bar">
      <div className="toolbar">
        {started(view) && (
          <button className={menu.panel === "Players" ? "on" : ""} title="Players" onClick={() => togglePanel("Players")}>
            <svg viewBox="0 0 24 24"><circle cx="9" cy="8" r="3.5" /><path d="M2 20c0-4 3-6 7-6s7 2 7 6" /><circle cx="17" cy="9" r="2.5" /><path d="M17 14c3 0 5 1.5 5 5" /></svg>
          </button>
        )}
        {canAutomate && (
          <button className={menu.panel === "Automation" ? "on" : ""} title="Automation" onClick={() => togglePanel("Automation")}>
            <svg viewBox="0 0 24 24"><rect x="5" y="8" width="14" height="11" rx="2.5" /><path d="M12 5v3" /><circle cx="12" cy="4" r="1" /><circle cx="9.5" cy="13" r=".6" fill="currentColor" /><circle cx="14.5" cy="13" r=".6" fill="currentColor" /><path d="M9.5 16.2h5M3 12.5v3M21 12.5v3" /></svg>
          </button>
        )}
        <button className={menuOpen || leaveOpen ? "on" : ""} title="Menu" onClick={toggleMenu}>
          <svg viewBox="0 0 24 24"><circle cx="5" cy="12" r="1.2" /><circle cx="12" cy="12" r="1.2" /><circle cx="19" cy="12" r="1.2" /></svg>
        </button>
        {tradeIcons(view) && (
          <>
            <span className="toolbar-sep"></span>
            <button className={menu.panel === "Deals" ? "on" : ""} title="Deals" onClick={() => togglePanel("Deals")}>
              <svg viewBox="0 0 24 24"><path d="M3 12l4-4 4 2 3-3 4 1 3 4-4 4" /><path d="M7 8l-4 4 5 5 2-1" /><path d="M11 13l2 2M13 11l3 3" /></svg>
              {dealsOffered > 0 && <span className="count">{dealsOffered}</span>}
            </button>
            {has("AllyPermission") && ally !== "None" && (
              <button className={menu.panel === "Alliance" ? "on" : ""} title={`Manage your Alliance with ${skin.faction(ally)}`} onClick={() => togglePanel("Alliance")}>
                <img className="ally" src={skin.factionImage(ally)} alt={skin.faction(ally)} />
              </button>
            )}
          </>
        )}
        {hasCardTools(view) && (
          <button className={menu.panel === "Cards" ? "on" : ""} title="Card Tracker" onClick={() => togglePanel("Cards")}>
            <svg viewBox="0 0 24 24"><rect x="4" y="6" width="10" height="14" rx="1.5" /><path d="M8 4h10.5A1.5 1.5 0 0 1 20 5.5V18" /></svg>
          </button>
        )}
      </div>

      <TradePopover />

      {tradeIcons(view) && <DealToasts />}

      {botsPaused && (
        <button className="badge spice" title="Bots are paused. Click to resume." onClick={() => setBotSpeed(1)}>Bots paused</button>
      )}

      {(menuOpen || leaveOpen) && <div className="menu-scrim" onClick={closeMenus}></div>}

      <ul className={`menu-overflow ${menuOpen ? "show" : ""}`}>
        <li>
          <button className="dropdown-item" onClick={() => setRulesOpen(!rulesOpen)}>
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7.5v.5" /></svg>Rules &amp; help<span className="trail"><span className="chev">{rulesOpen ? "▾" : "▸"}</span></span>
          </button>
        </li>
        {rulesOpen && (
          <li className="nested">
            <button className="dropdown-item" data-bs-toggle="modal" data-bs-target="#mentatModal" onClick={closeMenus}>List of Cards</button>
            <a className="dropdown-item" href="docs/Dune-Rulebook.pdf" target="_blank">Dune Rulebook</a>
            <a className="dropdown-item" href="docs/Dune-IxianAndTleilaxu-Rulebook.pdf" target="_blank">Ixian &amp; Tleilaxu Rulebook</a>
            <a className="dropdown-item" href="docs/Dune-ChoamAndRichese-Rulebook.pdf" target="_blank">CHOAM &amp; Richese Rulebook</a>
            <a className="dropdown-item" href="docs/Dune-EcazAndMoritani-Rulebook.pdf" target="_blank">Ecaz &amp; Moritani Rulebook</a>
            <a className="dropdown-item" href="docs/Dune-FAQ-Nov-2020.pdf" target="_blank">Dune FAQ</a>
            <a className="dropdown-item" href="https://www.gf9games.com/dune/classic-dune/" target="_blank">Buy boardgame from GF9</a>
            <a className="dropdown-item" href="https://dune.games" target="_blank">Dune Dashboard</a>
            <button className="dropdown-item" data-bs-toggle="modal" data-bs-target="#creditModal" onClick={closeMenus}>Credits</button>
          </li>
        )}

        <li>
          <button className="dropdown-item" onClick={() => setSoundOpen(!soundOpen)}>
            <svg viewBox="0 0 24 24"><path d="M4 9v6h4l5 4V5L8 9z" /><path d="M16 9a4 4 0 0 1 0 6" /></svg>Sound<span className="trail"><span className="chev">{soundOpen ? "▾" : "▸"}</span></span>
          </button>
        </li>
        <li className="sliders">
          <MusicComponent visible={soundOpen} />
        </li>

        <li>
          <button className="dropdown-item" onClick={toggleFullScreen}>
            <svg viewBox="0 0 24 24"><path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" /></svg>Fullscreen<span className="trail">{fullscreen && <span className="check"></span>}</span>
          </button>
        </li>
        <li>
          <a className="dropdown-item" href="https://github.com/ronaldossendrijver/treachery.online/issues" target="_blank" onClick={closeMenus}>
            <svg viewBox="0 0 24 24"><path d="M8 9a4 4 0 0 1 8 0v5a4 4 0 0 1-8 0z" /><path d="M12 9v9M4 13h4M16 13h4M5 7l3 2M19 7l-3 2M5 19l3-2M19 19l-3-2M10 5l-1-2M14 5l1-2" /></svg>Report a bug or suggest improvements<span className="trail"><svg className="ext" viewBox="0 0 24 24"><path d="M14 4h6v6M20 4l-9 9M18 14v6H4V6h6" /></svg></span>
          </a>
        </li>
        <li>
          <button className="dropdown-item" onClick={toggleLeave}>
            <svg viewBox="0 0 24 24"><path d="M10 4H5v16h5M14 8l4 4-4 4M18 12H9" /></svg>Leave game{" "}
          </button>
        </li>

        {canSave && (
          <>
            <li><hr className="dropdown-divider" /></li>
            {" "}<li><h6 className="dropdown-header">{view.viewer.isHost ? "Host" : "Admin"}</h6></li>{" "}
            <li>
              <button className="dropdown-item" onClick={() => void save()}>
                <svg viewBox="0 0 24 24"><path d="M5 4h11l3 3v13H5z" /><path d="M8 4v5h7V4M8 20v-6h8v6" /></svg>Save game{" "}
              </button>
            </li>
          </>
        )}

        {/* The file inputs stay in the page while the menu is closed: picking a file closes it. */}
        {hostTools && (
          <>
            <li>
              <label className="dropdown-item">
                <svg viewBox="0 0 24 24"><path d="M3 19V5h6l2 2h10v12z" /><path d="M12 16v-6M9 12.5 12 10l3 2.5" /></svg>Load game…{" "}
                <input hidden type="file" id="fileInput" onChange={loadGame} />
              </label>
            </li>
            {" "}<li>
              <label className="dropdown-item">
                <svg viewBox="0 0 24 24"><path d="M12 3a9 9 0 1 0 0 18c1.4 0 2-.8 2-1.8 0-1.3-1.2-1.6-1.2-2.7 0-.9.8-1.5 1.8-1.5H17a4 4 0 0 0 4-4c0-4.4-4-8-9-8z" /><circle cx="7.5" cy="11" r="1" /><circle cx="10" cy="7" r="1" /><circle cx="15" cy="7.5" r="1" /></svg>Apply skin…{" "}
                <input hidden type="file" id="skinInput" onChange={loadSkin} />
              </label>
            </li>
            {view.eventNumber > 1 && (
              <li>
                <button className="dropdown-item" onClick={openUndo}>
                  <svg viewBox="0 0 24 24"><path d="M9 14 4 9l5-5" /><path d="M4 9h10a6 6 0 0 1 0 12h-3" /></svg>Undo…{" "}
                </button>
              </li>
            )}
            {numberOfBots > 0 && (
              <li className="bots">
                <svg viewBox="0 0 24 24"><rect x="4" y="8" width="16" height="11" rx="2" /><path d="M12 8V4M9 13v1M15 13v1" /></svg><span>Bots</span>{" "}
                <span className="seg">
                  <button className={view.botsSpeed === 0 ? "on" : ""} title="Bots are paused" onClick={() => setBotSpeed(0)}>Pause</button>{" "}
                  <button className={view.botsSpeed === 1 ? "on" : ""} title="Bots at normal speed" onClick={() => setBotSpeed(1)}>1×</button>{" "}
                  <button className={view.botsSpeed === 2 ? "on" : ""} title="Bots at higher speed" onClick={() => setBotSpeed(2)}>2×</button>{" "}
                  <button className={view.botsSpeed === 4 ? "on" : ""} title="Bots at highest speed" onClick={() => setBotSpeed(4)}>4×</button>
                </span>
              </li>
            )}
          </>
        )}
      </ul>

      {leaveOpen && (
        <div className="pop menu-leave">
          <h6>Leave this game?</h6>
          <p>You'll return to the main screen.</p>
          <div className="actions">
            <button className="btn btn-secondary btn-sm" onClick={closeMenus}>Stay</button>
            <button className="btn btn-danger btn-sm" onClick={() => { closeMenus(); window.location.assign(BASE); }}>Leave</button>
          </div>
        </div>
      )}
    </div>

    <MentatComponent id="mentatModal" />
    <CreditComponent id="creditModal" />
    </>
  );
}
