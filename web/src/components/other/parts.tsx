// Pieces the "other events" forms (package 7e) share: the card every action form sits in, the
// draft hook, and the generic widgets those forms use that no other package has drawn yet
// (RadioComponent, SegmentedComponent, SelectNumberComponent, the multi-choice
// SelectFromImageComponent, CommandButton).
import { useId, useRef, useState, type ReactNode } from "react";
import type { ActionOptions } from "../../generated/ActionOptions";
import type { ActionView } from "../../generated/ActionView";
import type { Faction } from "../../generated/enums";
import type { FactionOption } from "../../generated/FactionOption";
import { treacheryCardPopup, withSpiceIcons } from "../../popups";
import type { Skin } from "../../skin";
import { useGame } from "../../state/context";
import type { GameEventDraft } from "../../state/source";
import { CollapseButton } from "../CollapseButton";
import { useServerDraft } from "../formDraft";
import { Image } from "../Image";
import { NumberPicker } from "../NumberPicker";
import type { ImageChoice } from "../SelectFromImage";

// GameEventComponentLayout's collapse state, kept per component type for the session
// (SavedCollapsedStateValues).
const collapsedByType = new Map<string, boolean>();

// GenericComponents/GameEventComponentLayout.razor. Without a header there is no header row and
// the body can't be collapsed. `startCollapsed` is `Collapsed="CollapseState.Collapsed"`; panels
// opened from a popover (TradePopover, CardPlayPopover) start open and don't fade in.
export function Layout({ type, header, urgent = false, startCollapsed = false, inPopover = false, children }: {
  type: string;
  header: ReactNode | null;
  urgent?: boolean;
  startCollapsed?: boolean;
  inPopover?: boolean;
  children: ReactNode;
}) {
  const [collapsed, setCollapsed] = useState(() => !inPopover && (startCollapsed || (collapsedByType.get(type) ?? false)));
  const toggle = () => {
    if (!startCollapsed) collapsedByType.set(type, !collapsed);
    setCollapsed(!collapsed);
  };
  const shown = header == null ? false : collapsed;
  return (
    <div className={`card ${urgent ? "card-urgent" : ""} mb-2`} style={inPopover ? undefined : { animation: "fadeIn 1s" }}>
      {header != null && (
        <div className="card-header" style={{ cursor: "pointer" }} onClick={toggle}>
          <CollapseButton collapsed={collapsed} />
          {header}
        </div>
      )}
      {!shown && <div className="card-body">{children}</div>}
    </div>
  );
}

type WithDraft = Extract<ActionOptions, { draft: Record<string, unknown> }>;

// A form's state (formDraft.ts): the server's latest options and validation, whose `draft` is the
// event with the inputs as the component shows them (rust/view/src/schema/other.rs).
export function useOtherDraft<F extends WithDraft["form"]>(action: ActionView, form: F) {
  const { source, options, draft, validation, update, error, setError } = useServerDraft(action, form);
  const submit = async (changes: Record<string, unknown> = {}) => setError(await source.submit({ ...draft, ...changes } as GameEventDraft));
  return { options, draft, validation, update, submit, error };
}

// The server's answer to a refused submission, under the form.
export function SubmitError({ error }: { error: string | null }) {
  return error ? <div className="text-danger small">{error}</div> : null;
}

export const num = (v: unknown): number => (typeof v === "number" ? v : Number(v ?? 0));

// The id lists of `_cardIds` and friends (GameEvent.IdStringToObjects): ids joined by ';'.
export function idList(v: unknown): number[] {
  return typeof v === "string" && v !== "" ? v.split(";").map(Number) : [];
}
export const idString = (ids: number[]): string => ids.join(";");

// Skin.Format("... {0} ...", value) for the client's own labels.
export function fmt(template: string, value: string): string {
  return template.replace("{0}", value);
}

// Skin.Describe(TreacheryCardType) and Skin.Describe(Concept).
export const cardType = (skin: Skin, t: string): string => skin.enumValue("TreacheryCardType", t);

// SelectFromImageComponent over treachery cards (by id): the card's rich popover.
export function cardChoices(skin: Skin, ids: number[]): ImageChoice<number>[] {
  return ids.map((id) => ({ value: id, src: skin.cardImage(id), alt: skin.card(id), popover: treacheryCardPopup(skin, id) }));
}

// SelectFromImageComponent over factions (by enum number): a faction has no rich popover, so
// Image.razor shows its name as a tooltip.
export function factionChoices(skin: Skin, factions: FactionOption[]): ImageChoice<number>[] {
  return factions.map((f) => ({ value: f.value, src: skin.factionImage(f.faction), alt: skin.faction(f.faction), popover: null, tooltip: skin.faction(f.faction) }));
}

// SelectFromImageComponent with `Selected`: several values, each click toggles one; shift-click
// keeps only the clicked one.
export function SelectManyFromImage<T>({ values, selected, onChange, imageWidth = 60, label }: {
  values: ImageChoice<T>[];
  selected: T[];
  onChange: (v: T[]) => void;
  imageWidth?: number;
  label?: ReactNode;
}) {
  // Image takes no event: the shift key is read from the mouse-down before the click.
  const shift = useRef(false);
  const click = (item: T) => {
    if (shift.current) onChange([item]);
    else onChange(selected.includes(item) ? selected.filter((s) => s !== item) : [...selected, item]);
  };
  return (
    <>
      {label != null && <label className="d-block mt-1 mb-0">{label}</label>}
      <div className="img-choices" onMouseDownCapture={(e) => { shift.current = e.shiftKey; }}>
        {values.map((c) => (
          <Image key={String(c.value)} src={c.src} alt={c.alt} width={imageWidth} popover={c.popover} tooltip={c.tooltip ?? null} pointer highlightOnHover
                 darkened={!selected.includes(c.value)} onClick={() => click(c.value)} />
        ))}
      </div>
    </>
  );
}

// GenericComponents/SelectNumberComponent.razor.
export function SelectNumber({ value, onChange, min, max, label = "" }: {
  value: number; onChange: (v: number) => void; min: number; max: number; label?: string;
}) {
  return (
    <div className="d-flex align-items-center">
      {label !== "" && <label className="me-1">{label}</label>}
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolFill="white" />
    </div>
  );
}

export interface Choice<T> {
  value: T;
  label: string;
}

// GenericComponents/RadioComponent.razor. Labels may hold spice amounts (Skin.WithSpiceIcons).
export function Radio<T>({ values, value, onChange, label }: {
  values: Choice<T>[]; value: T; onChange: (v: T) => void; label?: ReactNode;
}) {
  const { skin } = useGame();
  const id = useId();
  return (
    <>
      {label != null && <label className="mb-0" htmlFor={id}>{label}</label>}
      {values.map((c, i) => (
        <div key={i} className="form-check mb-0">
          <input className="form-check-input" style={{ cursor: "pointer" }} type="radio" id={`${id}-${i}`} value={String(c.value)}
                 checked={c.value === value} onChange={() => onChange(c.value)} />
          <label className="form-check-label" style={{ cursor: "pointer" }} htmlFor={`${id}-${i}`}
                 dangerouslySetInnerHTML={{ __html: withSpiceIcons(skin, c.label) }} />
        </div>
      ))}
    </>
  );
}

// GenericComponents/SegmentedComponent.razor: a row of buttons where one is on.
export function Segmented<T>({ values, value, onChange, label, className = "mt-1" }: {
  values: Choice<T>[]; value: T; onChange: (v: T) => void; label?: ReactNode; className?: string;
}) {
  const id = useId();
  return (
    <div className={className}>
      {label != null && <label className="mb-0 d-block" id={`${id}-label`}>{label}</label>}
      <div className="seg" role="radiogroup" aria-labelledby={label != null ? `${id}-label` : undefined}>
        {values.map((c, i) => (
          <button key={i} type="button" role="radio" aria-checked={c.value === value} className={c.value === value ? "on" : ""}
                  onClick={() => onChange(c.value)}>
            {c.label}
          </button>
        ))}
      </div>
    </div>
  );
}

// GenericComponents/CommandButton.razor.
export function CommandButton({ onClick, children, className = "btn-primary", small = false, margin = "me-1" }: {
  onClick: () => Promise<void> | void; children: ReactNode; className?: string; small?: boolean; margin?: string;
}) {
  const [busy, setBusy] = useState(false);
  return (
    <button className={`btn ${className} ${small ? "btn-sm" : ""} ${margin} `} disabled={busy}
            onClick={async () => { setBusy(true); try { await onClick(); } finally { setBusy(false); } }}>
      {children}
    </button>
  );
}

// The faction of the viewer.
export function useFaction(): Faction {
  const { view } = useGame();
  return (view.viewer.faction ?? "None") as Faction;
}
