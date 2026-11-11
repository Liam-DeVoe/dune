// When ImageFlash shows a flash. The C# component means to flash after every new event (it tracks
// afterEventId) but sets the flash's time only on its first render, so in C# a flash shows only
// when the game page opens with one current. The web client flashes after each new event that
// brings a flash, and on the first render when one is current (protocol.md, deviation 6).
//
// A view that is not a new event (a draft reply, chat, a reconnect's re-send, an undo back to an
// earlier event) has an eventNumber no higher than the last one seen, and never starts a flash.

export interface FlashState<F> {
  /** Increases with each flash started, so the component restarts its timer and animation. */
  key: number;
  /** The flashed items, kept while later views come in: a newer flash replaces them. */
  items: F[];
  /** How long the flash shows, in ms: 5000 + 1000 per item (ImageFlashComponent). */
  time: number;
  /** The eventNumber of the latest view. */
  lastEvent: number;
  /** The flash is the one current when the page opened, which C# shows too. */
  onOpen: boolean;
}

export function flashTime(items: number): number {
  return items > 0 ? 5000 + items * 1000 : 0;
}

/** The state for the first view the page shows: its flash, if it has one. */
export function initialFlash<F>(eventNumber: number, flash: F[]): FlashState<F> {
  return { key: flash.length > 0 ? 1 : 0, items: flash, time: flashTime(flash.length), lastEvent: eventNumber, onOpen: true };
}

/** The state after a view arrives. Returns `state` itself when nothing changes. */
export function nextFlash<F>(state: FlashState<F>, eventNumber: number, flash: F[]): FlashState<F> {
  if (eventNumber === state.lastEvent) return state;
  if (eventNumber < state.lastEvent || flash.length === 0) return { ...state, lastEvent: eventNumber };
  return { key: state.key + 1, items: flash, time: flashTime(flash.length), lastEvent: eventNumber, onOpen: false };
}
