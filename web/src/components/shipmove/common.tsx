// Shared pieces of the ship-and-move forms (client-plan.md package 7c): the event panel
// (GenericComponents/GameEventComponentLayout.razor with its CollapseButton), the draft and its
// queries (GameEventComponent's ConfirmedResult and Validation), the pinned map panel's parts
// (MapPickComponent.PinActions, MapForceStepper) and the savegame encodings drafts use.
import { useMemo, useRef, useState, type ReactNode } from "react";
import { useViewChanges } from "../formDraft";
import type { ActionOptions } from "../../generated/ActionOptions";
import type { ActionView } from "../../generated/ActionView";
import type { Faction } from "../../generated/enums";
import type { Part } from "../../generated/Part";
import { useGame } from "../../state/context";
import { Board } from "../board/geometry";
import { RichText } from "../RichText";

// GameEventComponentLayout's collapsed state, kept per form type for the page's lifetime
// (SavedCollapsedStateValues).
const collapsedByType = new Map<string, boolean>();

function CollapseButton({ collapsed }: { collapsed: boolean }) {
  return (
    <div className="btn btn-link p-0 float-end" style={{ display: "inline-block" }}>
      {collapsed ? (
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M13.825 7.15833L10 10.975L6.175 7.15833L5 8.33332L10 13.3333L15 8.33332L13.825 7.15833Z" fill="currentColor" />
        </svg>
      ) : (
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M10 6.66669L5 11.6667L6.175 12.8417L10 9.02502L13.825 12.8417L15 11.6667L10 6.66669Z" fill="currentColor" />
        </svg>
      )}
    </div>
  );
}

// GameEventComponentLayout: a card, urgent forms with the card-urgent border; the header
// collapses the body.
export function EventLayout({ type, urgent = false, header, children }: { type: string; urgent?: boolean; header?: ReactNode; children: ReactNode }) {
  const [collapsed, setCollapsed] = useState(() => collapsedByType.get(type) ?? false);
  const toggle = () => {
    collapsedByType.set(type, !collapsed);
    setCollapsed(!collapsed);
  };
  return (
    <div className={`card  ${urgent ? "card-urgent" : ""}  mb-2`} style={{ animation: "fadeIn 1s" }}>
      {header != null && (
        <div className="card-header" style={{ cursor: "pointer" }} onClick={toggle}>
          <CollapseButton collapsed={collapsed} />
          {header}
        </div>
      )}
      {!collapsed && <div className="card-body">{children}</div>}
    </div>
  );
}

// The validation message ExpressionComponent draws.
export function Validation({ parts, className = "pin-error" }: { parts: Part[] | null; className?: string }) {
  if (!parts) return null;
  return <div className={className}><RichText parts={parts} /></div>;
}

export type Draft = Record<string, unknown>;

// Narrows a form's options to its variant.
export function optionsOf<F extends ActionOptions["form"]>(o: ActionOptions | null | undefined, form: F): Extract<ActionOptions, { form: F }> | null {
  return o != null && o.form === form ? (o as Extract<ActionOptions, { form: F }>) : null;
}

// The draft a form's options carry: the event with the inputs as the C# component shows them,
// after the server made the component's render-time adjustments (rust/view/src/schema/ship_move.rs).
function draftOf(o: ActionOptions | null | undefined): Draft | null {
  const d = (o as { draft?: unknown } | null | undefined)?.draft;
  return d != null && typeof d === "object" ? (d as Draft) : null;
}

// Runs after the answer to a draft arrives, with the options for that draft and the adjusted
// draft: a click handler whose next step depends on the new options (InitializeAmounts after a
// destination is picked). Returns the next draft, or null to stop.
export type Then = (options: ActionOptions, draft: Draft) => Draft | null;

// The form's draft (the C# component's fields, as the event in savegame JSON) and the server's
// answer for it. `set` changes the draft and asks again; the answer replaces the draft with the
// server's adjusted one. Answers to older drafts are dropped, and the previous options stay on
// screen until the answer arrives. A new view keeps the draft and asks again for it
// (../formDraft.ts); forms whose C# component resets part of its input when the game is replaced
// (a map selection) do so themselves.
export function useDraft(action: ActionView, initial: () => Draft) {
  const { source } = useGame();
  const fromView = () => draftOf(action.options) ?? initial();
  const [draft, setDraftState] = useState<Draft>(fromView);
  const [options, setOptions] = useState<ActionOptions | null>(action.options);
  const [validation, setValidation] = useState<Part[] | null>(action.validation);
  const [failure, setFailure] = useState<string | null>(null);
  const latest = useRef(0);
  const current = useRef(draft);

  useViewChanges((change) => {
    setOptions(action.options);
    setValidation(action.validation);
    setFailure(null);
    if (change === "seat") {
      latest.current++;
      current.current = fromView();
      setDraftState(current.current);
      return;
    }
    void ask(current.current);
  });

  const ask = async (d: Draft, then?: Then) => {
    const n = ++latest.current;
    try {
      const r = await source.query({ $type: action.eventType, ...d });
      if (n !== latest.current) return;
      if (r.options != null) setOptions(r.options);
      setValidation(r.validation);
      const adjusted = draftOf(r.options) ?? d;
      if (adjusted !== d) {
        current.current = adjusted;
        setDraftState(adjusted);
      }
      const next = then && r.options != null ? then(r.options, adjusted) : null;
      if (next) set(next);
    } catch {
      // The connection reports its own errors; keep the previous answer.
    }
  };

  // Changes the draft and queries it. Takes changes or a function of the current draft, and
  // optionally the step to take once the answer arrives.
  const set = (changes: Draft | ((d: Draft) => Draft), then?: Then) => {
    const next = typeof changes === "function" ? changes(current.current) : { ...current.current, ...changes };
    current.current = next;
    setDraftState(next);
    void ask(next, then);
  };

  // Sends an event: the draft (Confirm) or another result of the component (Pass, Other).
  const send = async (event: Draft) => setFailure(await source.submit({ $type: action.eventType, ...event }));
  const confirm = () => send(current.current);
  // The latest draft, for handlers that run between renders.
  const get = () => current.current;

  return { draft, options, validation, failure, set, send, confirm, get };
}

// Board geometry for the forms (which sectors are drawn, where the player's token sits).
export function useBoardGeometry(): Board {
  const { view, skin } = useGame();
  return useMemo(() => new Board(view, skin), [view, skin]);
}

// Location → territory, from the static data.
export function useTerritoryOf(): (location: number) => number {
  const { skin } = useGame();
  return (l: number) => skin.locationInfo(l)?.territory ?? -1;
}

// PlacementEvent.ForceLocationsString: "location:forces|specialForces", comma separated, in
// the dictionary's insertion order.
export interface Forces { forces: number; specialForces: number }
export type ForceMap = Array<[number, Forces]>;

export function forceLocationsString(m: ForceMap): string {
  return m.map(([l, b]) => `${l}:${b.forces}|${b.specialForces}`).join(",");
}

// PlacementEvent.ParseForceLocations, for a draft's `_forceLocations`.
export function parseForceLocations(s: unknown): ForceMap {
  if (typeof s !== "string" || s === "") return [];
  return s.split(",").map((pair) => {
    const [l, amounts] = pair.split(":");
    const [f, sf] = (amounts ?? "").split("|");
    return [Number(l), { forces: Number(f) || 0, specialForces: Number(sf) || 0 }] as [number, Forces];
  });
}

// The member name of a savegame enum number (the inverse of the generated `*Value` tables).
export function enumName<T extends string>(values: Record<T, number>, n: unknown, fallback: NoInfer<T>): T {
  return (Object.keys(values) as T[]).find((k) => values[k] === n) ?? fallback;
}

// A savegame id field: -1 is null.
export function idOf(v: unknown): number | null {
  return typeof v === "number" && v >= 0 ? v : null;
}

// The faction's force and special force names (Skin.Describe(Player.Force)).
export function useForceNames(faction: Faction): { force: string; special: string } {
  const { skin } = useGame();
  const info = skin.statics.factions.find((f) => f.faction === faction);
  return {
    force: info ? skin.get("ForceName_STR", info.force) : "",
    special: info ? skin.get("SpecialForceName_STR", info.specialForce) : "",
  };
}

// GenericComponents/MapForceStepper.razor: a small counter for panels pinned on the map.
export function MapForceStepper({ value, onChange, min = 0, max, faction, special = false, spice = false, label }: {
  value: number;
  onChange: (v: number) => void;
  min?: number;
  max: number;
  faction: Faction;
  special?: boolean;
  spice?: boolean;
  label?: string;
}) {
  const { skin } = useGame();
  const names = useForceNames(faction);
  const src = spice
    ? skin.url(skin.str("Resource_URL"))
    : skin.url(skin.get(special ? "FactionSpecialForceImage_URL" : "FactionForceImage_URL", faction));
  const starred = !spice && special && (faction === "Fremen" || faction === "Emperor" || faction === "Ixian");
  const background = spice ? "transparent" : skin.factionColor(faction);
  const title = label ?? (spice ? skin.concept("Resource") : special ? names.special : names.force);
  const change = (delta: number) => {
    const next = Math.min(max, Math.max(min, value + delta));
    if (next !== value) onChange(next);
  };
  return (
    <div className="pin-step" title={title}>
      <span className="ic" style={{ background }}>
        {starred ? <span className="force-token starred" style={{ ["--d" as string]: "16px" }}><img src={src} alt="" /></span> : <img src={src} alt="" />}
      </span>
      <button type="button" tabIndex={-1} aria-label="Fewer" disabled={value <= min} onClick={() => change(-1)}>−</button>
      <span className="val">{value}</span>
      <button type="button" tabIndex={-1} aria-label="More" disabled={value >= max} onClick={() => change(1)}>+</button>
    </div>
  );
}

// MapPickComponent.PinActions: the reason the event is not valid yet, Cancel, and confirm.
export function PinActions({ validation, cancel, confirm, label }: { validation: Part[] | null; cancel: () => void; confirm: () => void; label: string }) {
  return (
    <>
      <Validation parts={validation} />
      <div className="pin-actions">
        <button type="button" className="btn btn-secondary" onClick={cancel}>Cancel</button>
        <button type="button" className="btn btn-primary" disabled={validation != null} onClick={confirm}>{label}</button>
      </div>
    </>
  );
}

// A row of mutually exclusive buttons in a pinned panel (.pin-choice).
export function PinChoice<T>({ values, value, label, onChange, wrap = false }: {
  values: T[];
  value: T;
  label: (v: T) => ReactNode;
  onChange: (v: T) => void;
  wrap?: boolean;
}) {
  return (
    <div className={`pin-choice ${wrap ? "wrap" : ""}`}>
      {values.map((v, i) => <button key={i} type="button" className={v === value ? "on" : ""} onClick={() => onChange(v)}>{label(v)}</button>)}
    </div>
  );
}
