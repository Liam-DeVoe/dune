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
import { useActCard } from "../ActStack";
import { CollapseButton } from "../CollapseButton";
import { type Fields, useServerDraft } from "../formDraft";
import { Image } from "../Image";
import { NumberPicker } from "../NumberPicker";
import type { ImageChoice } from "../SelectFromImage";

// GameEventComponentLayout's collapse state, kept per component type for the session
// (SavedCollapsedStateValues).
const collapsedByType = new Map<string, boolean>();

// GameEventComponentLayout. Without a header there is no header row and
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
  const act = useActCard();
  return (
    <div className={`card ${urgent ? "card--urgent" : ""} ${act} event-form`} style={inPopover ? undefined : { animation: "fadeIn 1s" }}>
      {header != null && (
        <div className="card__header card__header--collapsible" style={{ cursor: "pointer" }} onClick={toggle}>
          <CollapseButton collapsed={collapsed} />
          {header}
        </div>
      )}
      {!shown && <div className="card__body">{children}</div>}
    </div>
  );
}

type WithDraft = Extract<ActionOptions, { draft: Record<string, unknown> }>;

// A form's state (formDraft.ts): the server's latest options and validation, whose `draft` is the
// event with the inputs as the component shows them (rust/view/src/schema/other.rs).
export function useOtherDraft<F extends WithDraft["form"]>(action: ActionView, form: F) {
  const { source, options, draft, validation, update, withChanges, pass, error, setError } = useServerDraft(action, form);
  // Confirm sends the draft, with some inputs changed for buttons that set one. Pass sends the
  // pass of a passable event.
  const submit = async (changes: Fields = {}) => setError(await source.submit(withChanges(changes)));
  const submitPass = async () => setError(await source.submit(pass()));
  return { options, draft, validation, update, submit, submitPass, error };
}

// The server's answer to a refused submission, under the form.
export function SubmitError({ error }: { error: string | null }) {
  return error ? <div className="event-form__error">{error}</div> : null;
}

export const num = (v: unknown): number => (typeof v === "number" ? v : Number(v ?? 0));

// An id list input (`cardIds` and friends).
export function idList(v: unknown): number[] {
  return Array.isArray(v) ? v.filter((x): x is number => typeof x === "number") : [];
}

// An id input: null for none.
export const idOf = (v: unknown): number | null => (typeof v === "number" && v >= 0 ? v : null);

// A faction input: null for none.
export const factionOf = (v: unknown): Faction | null => (typeof v === "string" && v !== "None" ? (v as Faction) : null);

// An enum input other than a faction: its member name, null for none.
export const nameOf = (v: unknown): string | null => (typeof v === "string" ? v : null);

export function fmt(template: string, value: string): string {
  return template.replace("{0}", value);
}

// The names of a treachery card type and of a concept.
export const cardType = (skin: Skin, t: string): string => skin.enumValue("TreacheryCardType", t);

// SelectFromImageComponent over treachery cards (by id): the card's rich popover.
export function cardChoices(skin: Skin, ids: number[]): ImageChoice<number>[] {
  return ids.map((id) => ({ value: id, src: skin.cardImage(id), alt: skin.card(id), popover: treacheryCardPopup(skin, id) }));
}

// SelectFromImageComponent over factions: a faction has no rich popover, so Image shows its
// name as a tooltip.
export function factionChoices(skin: Skin, factions: FactionOption[]): ImageChoice<Faction>[] {
  return factions.map((f) => ({ value: f.faction, src: skin.factionImage(f.faction), alt: skin.faction(f.faction), popover: null, tooltip: skin.faction(f.faction) }));
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
      {label != null && <label className="select-from-image__label">{label}</label>}
      <div className="img-choices" onMouseDownCapture={(e) => { shift.current = e.shiftKey; }}>
        {values.map((c) => (
          <Image key={String(c.value)} src={c.src} alt={c.alt} width={imageWidth} popover={c.popover} tooltip={c.tooltip ?? null} pointer highlightOnHover
                 darkened={!selected.includes(c.value)} onClick={() => click(c.value)} />
        ))}
      </div>
    </>
  );
}

export function SelectNumber({ value, onChange, min, max, label = "" }: {
  value: number; onChange: (v: number) => void; min: number; max: number; label?: string;
}) {
  return (
    <div className="event-form__stepper event-form__stepper--block">
      {label !== "" && <label className="event-form__stepper-label">{label}</label>}
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolFill="white" />
    </div>
  );
}

export interface Choice<T> {
  value: T;
  label: string;
}

// RadioComponent. Labels may hold spice amounts.
export function Radio<T>({ values, value, onChange, label }: {
  values: Choice<T>[]; value: T; onChange: (v: T) => void; label?: ReactNode;
}) {
  const { skin } = useGame();
  const id = useId();
  return (
    <>
      {label != null && <label htmlFor={id}>{label}</label>}
      {values.map((c, i) => (
        <div key={i} className="check event-form__radio">
          <input className="check__input" style={{ cursor: "pointer" }} type="radio" id={`${id}-${i}`} value={String(c.value)}
                 checked={c.value === value} onChange={() => onChange(c.value)} />
          <label className="check__label" style={{ cursor: "pointer" }} htmlFor={`${id}-${i}`}
                 dangerouslySetInnerHTML={{ __html: withSpiceIcons(skin, c.label) }} />
        </div>
      ))}
    </>
  );
}

// A row of buttons where one is on.
export function Segmented<T>({ values, value, onChange, label, className = "event-form__field" }: {
  values: Choice<T>[]; value: T; onChange: (v: T) => void; label?: ReactNode; className?: string;
}) {
  const id = useId();
  return (
    <div className={className}>
      {label != null && <label className="event-form__seg-label" id={`${id}-label`}>{label}</label>}
      <div className="seg" role="radiogroup" aria-labelledby={label != null ? `${id}-label` : undefined}>
        {values.map((c, i) => (
          <button key={i} type="button" role="radio" aria-checked={c.value === value} className={c.value === value ? "seg__button seg__button--on" : "seg__button"}
                  onClick={() => onChange(c.value)}>
            {c.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function CommandButton({ onClick, children, className = "button--primary", small = false, margin = "event-form__command" }: {
  onClick: () => Promise<void> | void; children: ReactNode; className?: string; small?: boolean; margin?: string;
}) {
  const [busy, setBusy] = useState(false);
  return (
    <button className={`button ${className} ${small ? "button--small" : ""} ${margin} `} disabled={busy}
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
