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
import type { Part } from "../generated/Part";
import { useGame } from "../state/context";
import type { GameEventDraft } from "../state/source";

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
// the C# component shows them, after the server's render-time adjustments). An input change shows
// at once and asks the server again; answers to older queries are dropped. A new view keeps the
// draft and asks for it again (see the top of this file).
export function useServerDraft<F extends WithDraft["form"]>(action: ActionView, form: F) {
  const { source } = useGame();
  type O = Extract<ActionOptions, { form: F }>;
  const fromView = () => ({ options: action.options as O | null, validation: action.validation });
  const [result, setResult] = useState(fromView);
  const [error, setError] = useState<string | null>(null);
  const latest = useRef(0);
  // Cast: TypeScript can't read `form` through Extract over the whole union for a generic F.
  const options = (result.options as ActionOptions | null)?.form === form ? (result.options as O) : null;
  const draft = ((options as { draft?: Record<string, unknown> } | null)?.draft ?? { $type: action.eventType }) as GameEventDraft;
  const kept = useRef<GameEventDraft | null>(null);
  kept.current = options ? draft : null;

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

  const update = async (changes: Record<string, unknown>) => {
    const next = { ...draft, ...changes } as GameEventDraft;
    if (options) setResult({ options: { ...options, draft: next }, validation: result.validation });
    const n = ++latest.current;
    const r = await source.query(next);
    if (n === latest.current && r.options?.form === form) setResult({ options: r.options as O, validation: r.validation });
  };

  return { source, options, draft, validation: result.validation as Part[] | null, update, error, setError };
}
