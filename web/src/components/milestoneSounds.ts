// Client.PlaySoundsForMilestones: after each event, the sound of every milestone it reached
// (`view.recentMilestones`, Skin.GetSound), at the effect volume. C# plays them in
// PerformPostEventTasks, which runs after an event, an undo and a (re)load of the game; so they
// play for the first view and for every view of another event number, not for views the server
// resends for the same event (participation, timer, skin).
import { useEffect, useRef } from "react";
import type { GameView } from "../generated/GameView";
import type { Skin } from "../skin";
import { playSound, soundSettings } from "../sound";

export function useMilestoneSounds(view: GameView, skin: Skin): void {
  const played = useRef<number | null>(null);
  useEffect(() => {
    if (played.current === view.eventNumber) return;
    played.current = view.eventNumber;
    for (const m of view.recentMilestones) playSound(skin.get("Sound", m), soundSettings.effect);
  }, [view, skin]);
}
