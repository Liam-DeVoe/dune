// What every action form does with its inputs when a new view arrives. A C# component keeps its
// fields for as long as it stays on screen: a game update redraws it but doesn't reset it. The
// forms here do the same. React keeps a form mounted while ActionPanel renders it under the same
// key (its event type), and the form keeps its draft across views. After each view the form
// sends the kept draft back as a draft query, so the server adjusts it to the new game (an input
// that no longer applies is dropped) and validates it again. A kept draft therefore never goes
// stale without the server seeing it.
//
// A draft is dropped, and the form starts over from the view's options, when the form leaves the
// screen (it unmounts), when the server's answer is for another form, or when the viewer's seat
// changes (the inputs were another player's). Forms whose C# component resets part of its input
// when the game is replaced (undo or load: ShipmentComponent's and PlacementComponent's
// ResetSelection) do that themselves on a "replaced" change.
import { useEffect, useRef, useState } from "react";
import type { ActionOptions } from "../generated/ActionOptions";
import type { ActionView } from "../generated/ActionView";
import type { Event } from "../generated/Event";
import type { Part } from "../generated/Part";
import { useGame } from "../state/context";

// Events in drafts and submissions are in the engine's wire format (generated/Event.ts). An event
// that may be passed holds its inputs in `play`, which is null for a pass. Other events hold
// their inputs next to `type`.
type Passable = Extract<Event, { play: unknown }>;
export type PassableType = Passable["type"];

const PASSABLE = [
  "AllianceByAmbassador", "AllianceByTerror", "AmbassadorActivated", "BattleClaimed", "Bid", "BlackMarketBid", "Bureaucracy",
  "Captured", "Caravan", "CardGiven", "DiscoveryEntered", "DiscoveryRevealed", "DivideResources", "DivideResourcesAccepted",
  "FaceDanced", "FaceDancerReplaced", "FaceDancerRevealed", "FremenRidesMonster", "IxianSwappedCardOnBid", "KarmaHmsMovement",
  "Move", "MulliganPerformed", "NexusCardDrawn", "NexusVoted", "PerformFremenSetup", "PerformHmsMovement", "PerformSetup",
  "ReplacedCardWon", "RicheseAnnouncesBlackMarket", "RicheseKeepsUnsoldCard", "Shipment", "SkillAssigned", "TerrorPlanted",
  "TerrorRevealed",
] as const satisfies readonly PassableType[];
// Fails to compile when Event.ts gains a passable event that PASSABLE lacks.
const passableComplete: [Exclude<PassableType, (typeof PASSABLE)[number]>] extends [never] ? true : false = true;
void passableComplete;

export function isPassable(type: string): type is PassableType {
  return (PASSABLE as readonly string[]).includes(type);
}

// The inputs of one event type: `play` of a passable event, the fields beside `type` otherwise.
type InputsOf<E> = E extends { play: infer P } ? NonNullable<P> : Omit<E, "type">;
// Every input name of every event. Forms keep their inputs as `Fields`, so a misspelt or
// old-format name (`_toId`, `Passed`) is a type error.
export type FieldName = Event extends infer E ? (E extends Event ? Extract<keyof InputsOf<E>, string> : never) : never;
export type Fields = { [K in FieldName]?: unknown };

// The inputs of an event. A pass has none.
export function fieldsOf(event: Event | Record<string, unknown>): Fields {
  if ("play" in event) return (event.play ?? {}) as Fields;
  const rest: Record<string, unknown> = { ...event };
  delete rest.type;
  return rest as Fields;
}

// The event of type `type` with the inputs `fields`.
export function eventOf(type: string, fields: Fields): Event {
  return (isPassable(type) ? { type, play: fields } : { type, ...fields }) as Event;
}

// The pass of a passable event.
export function passOf(type: string): Event {
  return { type, play: null } as Event;
}

// How a view relates to the one the form saw before it:
// - "event": the game moved on (or only the participation changed). The form keeps its inputs.
// - "replaced": the event number went back, so the game was replaced (an undo or a load).
// - "seat": the viewer plays another faction now. The inputs were for the old seat.
export type ViewChange = "event" | "replaced" | "seat";

// Runs `onChange` after each view that follows the first one the form saw, while it stays mounted.
export function useViewChanges(onChange: (change: ViewChange) => void): void {
  const { view } = useGame();
  const seen = useRef({ view, eventNumber: view.eventNumber, seat: view.viewer.faction });
  const handler = useRef(onChange);
  handler.current = onChange;
  useEffect(() => {
    const before = seen.current;
    if (before.view === view) return;
    seen.current = { view, eventNumber: view.eventNumber, seat: view.viewer.faction };
    handler.current(before.seat !== view.viewer.faction ? "seat" : view.eventNumber < before.eventNumber ? "replaced" : "event");
  }, [view]);
}

type WithDraft = Extract<ActionOptions, { draft: Record<string, unknown> }>;

// The state of a form whose options carry its draft (`options.draft`: the event with the inputs as
// the C# component shows them, after the server's render-time adjustments). `draft` holds the
// draft's inputs (`fieldsOf`). An input change shows at once and asks the server again; answers to
// older queries are dropped. A new view keeps the draft and asks for it again (see the top of
// this file).
export function useServerDraft<F extends WithDraft["form"]>(action: ActionView, form: F) {
  const { source } = useGame();
  type O = Extract<ActionOptions, { form: F }>;
  const fromView = () => ({ options: action.options as O | null, validation: action.validation });
  const [result, setResult] = useState(fromView);
  const [error, setError] = useState<string | null>(null);
  const latest = useRef(0);
  // Cast: TypeScript can't read `form` through Extract over the whole union for a generic F.
  const options = (result.options as ActionOptions | null)?.form === form ? (result.options as O) : null;
  const event = ((options as { draft?: Record<string, unknown> } | null)?.draft as Event | undefined) ?? eventOf(action.eventType, {});
  const draft = fieldsOf(event);
  const kept = useRef<Event | null>(null);
  kept.current = options ? event : null;

  useViewChanges((change) => {
    setError(null);
    const n = ++latest.current;
    const d = kept.current;
    if (change === "seat" || d == null) {
      setResult(fromView());
      return;
    }
    source.query(d).then((r) => {
      if (n !== latest.current) return;
      setResult(r.options?.form === form ? { options: r.options as O, validation: r.validation } : fromView());
    }).catch(() => {
      // The connection reports its own errors; keep the previous answer.
    });
  });

  // Changes some inputs and asks the server again. Changing an input of a pass starts from no
  // inputs.
  const update = async (changes: Fields) => {
    const next = eventOf(event.type, { ...draft, ...changes });
    if (options) setResult({ options: { ...options, draft: next }, validation: result.validation });
    const n = ++latest.current;
    const r = await source.query(next);
    if (n === latest.current && r.options?.form === form) setResult({ options: r.options as O, validation: r.validation });
  };

  // The draft with some inputs changed, and the pass of the draft's event, to submit.
  const withChanges = (changes: Fields = {}): Event => eventOf(event.type, { ...draft, ...changes });
  const pass = (): Event => passOf(event.type);

  return { source, options, draft, validation: result.validation as Part[] | null, update, withChanges, pass, error, setError };
}
