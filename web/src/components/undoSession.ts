// UndoSession.cs: the host choosing a moment to undo to. A moment is a gap between two events:
// gap n sits just before event n, so undoing to it is RequestUndo(n), which keeps events 0 to
// n-1. The undo panel (UndoComponent) opens and closes the session; the timeline under the map
// (UndoTimeline) moves the gap.
//
// While a moment other than now is chosen, the game screen shows the preview: the host's view at
// that gap, without actions (RequestUndoPreview), as C# showed UndoSession.Preview. The previous
// preview stays shown until the next one arrives.
import { useSyncExternalStore } from "react";
import type { GameView } from "../generated/GameView";
import type { UndoEntry } from "../state/source";

export interface UndoState {
  // The undo list (RequestUndoList), or null while it loads.
  entries: UndoEntry[] | null;
  // Null while the chosen moment is now.
  gap: number | null;
  // The view at `gap` (or at the gap chosen before, while it loads); null at now.
  preview: GameView | null;
}

// Event 0 sets the game up and can't be undone.
export const FIRST_GAP = 1;

let state: UndoState | null = null;
let loadPreview: ((gap: number) => Promise<GameView>) | null = null;
const listeners = new Set<() => void>();
const set = (s: UndoState | null) => { state = s; listeners.forEach((l) => l()); };

export const undoSession = {
  get: () => state,
  subscribe(l: () => void) {
    listeners.add(l);
    return () => { listeners.delete(l); };
  },
  // `preview` asks the server for the view at a gap; without it there is no preview.
  open: (load: () => Promise<UndoEntry[]>, preview?: (gap: number) => Promise<GameView>) => {
    loadPreview = preview ?? null;
    set({ entries: null, gap: null, preview: null });
    load().then((entries) => { if (state) set({ ...state, entries }); }).catch(() => { if (state) set({ ...state, entries: [] }); });
  },
  close: () => { loadPreview = null; set(null); },
  // UndoSession.MoveTo: clamped to the events there are; the last gap is now.
  moveTo(gap: number) {
    if (!state?.entries) return;
    const count = state.entries.length;
    const g = Math.min(Math.max(gap, FIRST_GAP), count);
    const gap2 = g === count ? null : g;
    if (gap2 === state.gap) return;
    set({ ...state, gap: gap2, preview: gap2 === null ? null : state.preview });
    if (gap2 !== null && loadPreview) {
      // Only the answer for the gap still chosen is shown.
      loadPreview(gap2).then((view) => { if (state && state.gap === gap2) set({ ...state, preview: view }); }).catch(() => {});
    }
  },
};

export function useUndoSession(): UndoState | null {
  return useSyncExternalStore(undoSession.subscribe, undoSession.get);
}

// The events that can be undone and are listed, oldest first (UndoSession.ListedEvents).
export function listedEvents(entries: UndoEntry[]): number[] {
  return entries.filter((e) => e.eventNumber >= FIRST_GAP && e.listed).map((e) => e.eventNumber);
}
