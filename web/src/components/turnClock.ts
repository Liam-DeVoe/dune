// TurnClock.cs: one clock for every "how long has the game been waiting" display (the turn
// banner and the prompt's timer), so they show the same time and change together. It ticks on
// whole seconds; `now` is the time of the latest tick.
import { useSyncExternalStore } from "react";

const floor = (t: number) => t - (t % 1000);

let now = floor(Date.now());
const listeners = new Set<() => void>();
let timer: ReturnType<typeof setTimeout> | null = null;

function tick() {
  now = floor(Date.now());
  listeners.forEach((l) => l());
  timer = setTimeout(tick, 1000 - (Date.now() % 1000));
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  if (timer === null) timer = setTimeout(tick, 1000 - (Date.now() % 1000));
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };
}

// The clock's time in milliseconds, redrawing the caller on every tick.
export function useClock(): number {
  return useSyncExternalStore(subscribe, () => now);
}

// TurnClock.Since: how long since `since`, as m:ss, with all the hours in front once there are any.
export function since(clock: number, from: string): string {
  let t = Math.floor((clock - Date.parse(from)) / 1000);
  if (t < 0) t = 0;
  const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = t % 60;
  const ss = String(s).padStart(2, "0");
  return h >= 1 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

// The seconds since `since` on the clock, as C#'s TimeSpan.TotalSeconds.
export function secondsSince(clock: number, from: string): number {
  return (clock - Date.parse(from)) / 1000;
}
