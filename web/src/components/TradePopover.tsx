// OtherComponents/TradePopover.razor: what the trade icons in the menu bar open, in a popover above
// the bar. Deals, Offer a Deal and Give Spice to a faction as tabs; the alliance; or the Card
// Tracker and the notekeeping assistant. The panels are the ones the action column used to show,
// opened up. A click outside, Escape, or the icon again closes it.
import { useEffect, useState } from "react";
import { PhaseValue } from "../generated/enums";
import type { GameView } from "../generated/GameView";
import { useGame } from "../state/context";
import { ActionPanel } from "./ActionPanel";
import { DealComponent } from "./DealComponent";
import { menu, useMenu, type MenuPanel } from "./menuState";
import { NoteComponent } from "./NoteComponent";
import { TrackerComponent } from "./TrackerComponent";

type DealTab = "Deals" | "Offer" | "Give";

const isOpen = (panel: MenuPanel) => panel === "Deals" || panel === "Alliance" || panel === "Cards";

// HasCardTools: whether the card icon has anything to open.
export function hasCardTools(view: GameView): boolean {
  return !view.viewer.isObserver && view.me !== null && view.eventNumber > 0
    && (view.notes !== null || (PhaseValue[view.phase] >= PhaseValue.SelectingTraitors && view.me.faction === "Atreides"));
}

export function TradePopover() {
  const { view, skin } = useGame();
  const m = useMenu();
  const [tab, setTab] = useState<DealTab>("Deals");
  const open = isOpen(m.panel);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") menu.close(); };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);

  if (!open) return null;
  const has = (t: string) => view.actions.some((a) => a.eventType === t);
  const d = view.deals;
  const tabs: DealTab[] = [];
  if (d && (d.current.length > 0 || d.acceptable.length > 0 || d.cancellable.length > 0)) tabs.push("Deals");
  if (has("DealOffered")) tabs.push("Offer");
  if (has("Donated")) tabs.push("Give");
  // Opened from a deal's toast, Deals shows with that offer marked.
  const current = m.markedDeal !== null && tabs.includes("Deals") ? "Deals" : tabs.includes(tab) ? tab : tabs[0];
  const tabName = (t: DealTab) => t === "Deals" ? "Deals" : t === "Offer" ? "Offer a Deal" : `Give ${skin.concept("Resource")}`;

  return (
    <>
      <div className="menu-scrim" onClick={menu.close}></div>
      <div className="pop trade-pop">
        {m.panel === "Deals"
          ? (
            <>
              {tabs.length > 1 && (
                <div className="pop-tabs">
                  {tabs.map((t) => <button key={t} className={t === current ? "on" : ""} onClick={() => setTab(t)}>{tabName(t)}</button>)}
                </div>
              )}
              <div className="pop-body">
                {current === "Deals" ? <DealComponent open />
                  : current === "Offer" ? <ActionPanel filter={(a) => a.eventType === "DealOffered"} inPopover />
                  : current === "Give" ? <ActionPanel filter={(a) => a.eventType === "Donated"} inPopover />
                  : <p className="pop-empty">No deals.</p>}
              </div>
            </>
          )
          : m.panel === "Alliance"
            ? <div className="pop-body"><ActionPanel filter={(a) => a.eventType === "AllyPermission"} inPopover /></div>
            : (
              <div className="pop-body">
                {view.notes !== null && <NoteComponent open />}
                {PhaseValue[view.phase] >= PhaseValue.SelectingTraitors && view.viewer.faction === "Atreides" && <TrackerComponent open />}
              </div>
            )}
      </div>
    </>
  );
}
