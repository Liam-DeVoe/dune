// MenuState.cs: what the menu bar under the map has open. The bar (MenuComponent) and the panels
// it opens at the top of the side column (MenuPanels) are in different columns, so they share
// this store instead of one holding the other. UI state only; nothing here comes from the view.
import { useSyncExternalStore } from "react";

export type MenuPanel = "None" | "Players" | "Undo" | "Automation"
  // These open in a popover above the menu bar (TradePopover), not in the side column.
  | "Deals" | "Alliance" | "Cards";

export interface MenuSnapshot {
  panel: MenuPanel;
  // Set when a saved game or skin fails to load.
  loadError: string;
  // The deal offer (its index in the view's offers) whose toast was clicked, marked in Deals
  // until the popover closes.
  markedDeal: number | null;
  // The treachery card whose play panel is open beside the faction panel (CardPlayPopover),
  // and the height, in window pixels, it was clicked at.
  cardPlay: number | null;
  cardPlayY: number;
}

let state: MenuSnapshot = { panel: "None", loadError: "", markedDeal: null, cardPlay: null, cardPlayY: 0 };
const listeners = new Set<() => void>();

function set(change: Partial<MenuSnapshot>) {
  state = { ...state, ...change };
  listeners.forEach((l) => l());
}

export const menu = {
  get: () => state,
  subscribe(l: () => void) {
    listeners.add(l);
    return () => { listeners.delete(l); };
  },
  toggle: (panel: MenuPanel) => set({ panel: state.panel === panel ? "None" : panel, markedDeal: null }),
  open: (panel: MenuPanel) => set({ panel }),
  close: () => set({ panel: "None", markedDeal: null }),
  setLoadError: (loadError: string) => set({ loadError }),
  markDeal: (markedDeal: number | null) => set({ markedDeal }),
  toggleCardPlay: (card: number, y: number) => set({ cardPlay: state.cardPlay === card ? null : card, cardPlayY: y }),
  closeCardPlay: () => set({ cardPlay: null }),
};

export type Menu = MenuSnapshot & Omit<typeof menu, "get" | "subscribe">;

// The menu state, redrawing the caller when it changes.
export function useMenu(): Menu {
  const s = useSyncExternalStore(menu.subscribe, menu.get);
  return { ...s, ...menu };
}
