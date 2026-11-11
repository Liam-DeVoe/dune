// Pieces the battle forms (package 7b) share: the card every action form sits in, the draft
// hook, radio buttons, and images of cards, heroes and tokens.
import { useId, useState, type ReactNode } from "react";
import type { ActionOptions } from "../../generated/ActionOptions";
import type { ActionView } from "../../generated/ActionView";
import type { BattleFormHero } from "../../generated/BattleFormHero";
import type { Faction } from "../../generated/enums";
import type { GameView } from "../../generated/GameView";
import type { HeroRef } from "../../generated/HeroRef";
import type { Part } from "../../generated/Part";
import { leaderPopup, messiahPopup, treacheryCardPopup } from "../../popups";
import type { Skin } from "../../skin";
import { useGame } from "../../state/context";
import type { GameEventDraft } from "../../state/source";
import { CollapseButton } from "../CollapseButton";
import { useServerDraft } from "../formDraft";
import { hasStarredSpecialForce } from "../ForceToken";
import { NumberPicker } from "../NumberPicker";
import { RichText } from "../RichText";
import type { ImageChoice } from "../SelectFromImage";

// GameEventComponentLayout's collapse state, kept per component type for the session.
const collapsedByType = new Map<string, boolean>();

// GenericComponents/GameEventComponentLayout.razor: a card with a collapsible header. Panels
// opened from a popover (the hand's card plays) start open and don't fade in.
export function Layout({ type, header, urgent = false, inPopover = false, className = "", children }: {
  type: string;
  header: ReactNode;
  urgent?: boolean;
  inPopover?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const [collapsed, setCollapsed] = useState(() => !inPopover && (collapsedByType.get(type) ?? false));
  const toggle = () => {
    collapsedByType.set(type, !collapsed);
    setCollapsed(!collapsed);
  };
  const classes = ["card", urgent ? "card-urgent" : "", className, "mb-2"].filter((c) => c).join(" ");
  return (
    <div className={classes} style={inPopover ? undefined : { animation: "fadeIn 1s" }}>
      <div className="card-header" style={{ cursor: "pointer" }} onClick={toggle}>
        <CollapseButton collapsed={collapsed} />
        {header}
      </div>
      {!collapsed && <div className="card-body">{children}</div>}
    </div>
  );
}

type WithDraft = Extract<ActionOptions, { draft: Record<string, unknown> }>;

// A battle form's state (formDraft.ts): the server's latest options and validation. Their `draft`
// is the event with the inputs as the component shows them (rust/view/src/schema/battle.rs).
export function useBattleDraft<F extends WithDraft["form"]>(action: ActionView, form: F) {
  const { source, options, draft, validation, update, error, setError } = useServerDraft(action, form);
  // Pass and Other send the draft with a field changed (PassedResult, OtherResult).
  const submit = async (changes: Record<string, unknown> = {}) => setError(await source.submit({ ...draft, ...changes } as GameEventDraft));
  // A submission the form makes by itself (BattleInitiated with one fight, TreacheryCalled when
  // there is nothing to call): C# only logs a refusal to the console.
  const autoSubmit = async (changes: Record<string, unknown> = {}) => {
    const refused = await source.submit({ ...draft, ...changes } as GameEventDraft);
    if (refused) console.log(refused);
  };
  return { options, draft, validation, update, submit, autoSubmit, error };
}

// The server's answer to a refused submission, under the form.
export function SubmitError({ error }: { error: string | null }) {
  return error ? <div className="text-danger small">{error}</div> : null;
}

// GenericComponents/RadioComponent.razor: one radio per value, with an optional label above.
export function Radio<T>({ values, labels, value, onChange, label }: {
  values: T[];
  labels: string[];
  value: T;
  onChange: (v: T) => void;
  label?: ReactNode;
}) {
  const id = useId();
  return (
    <>
      {label != null && <label className="mb-0" htmlFor={id}>{label}</label>}
      {values.map((v, i) => (
        <div key={String(v)} className="form-check mb-0">
          <input className="form-check-input" style={{ cursor: "pointer" }} type="radio" id={`${id}-${i}`} value={String(v)}
                 checked={v === value} onChange={() => onChange(v)} />
          <label className="form-check-label" style={{ cursor: "pointer" }} htmlFor={`${id}-${i}`}>{labels[i]}</label>
        </div>
      ))}
    </>
  );
}

// GenericComponents/SelectForcesComponent.razor: a number picker with the faction's force or
// special force token as its symbol.
export function SelectForces({ value, onChange, onStep, min, max, faction, special, label = "" }: {
  value: number;
  onChange: (v: number) => void;
  onStep?: (delta: number) => void;
  min: number;
  max: number;
  faction: Faction;
  special: boolean;
  label?: string;
}) {
  const { skin } = useGame();
  const src = skin.url(skin.get(special ? "FactionSpecialForceImage_URL" : "FactionForceImage_URL", faction));
  return (
    <div className="d-inline-flex align-items-center">
      {label !== "" && <label className="me-1">{label}</label>}
      <NumberPicker value={value} onChange={onChange} onStep={onStep} min={min} max={max} symbolSrc={src}
                    symbolStarred={special && hasStarredSpecialForce(faction)} symbolFill={skin.factionColor(faction)} />
    </div>
  );
}

// Skin.GetImageUrl(IHero).
export function heroImage(skin: Skin, h: HeroRef): string {
  if (h.t === "leader") return skin.personImage(h.id);
  if (h.t === "messiah") return skin.url(skin.str("Messiah_URL"));
  return skin.cardImage(h.id);
}

// Skin.GetPopup(IHero, Game): a skilled leader shows its skill card.
export function heroPopup(skin: Skin, view: GameView, h: HeroRef): string {
  if (h.t === "treacheryCard") return treacheryCardPopup(skin, h.id);
  if (h.t === "messiah") return messiahPopup(skin);
  const skill = view.players.flatMap((p) => p.skilledLeaders).find((s) => s.id === h.id)?.skill ?? null;
  return leaderPopup(skin, h.id, skill);
}

// SelectFromImageComponent's images over heroes, valued by their id.
export function heroChoices(skin: Skin, view: GameView, heroes: BattleFormHero[]): ImageChoice<number>[] {
  return heroes.map((h) => ({ value: h.id, src: heroImage(skin, h.hero), alt: h.label, popover: heroPopup(skin, view, h.hero) }));
}

// SelectFromImageComponent's images over treachery cards, valued by their id.
export function cardChoices(skin: Skin, cards: number[]): ImageChoice<number>[] {
  return cards.map((id) => ({ value: id, src: skin.cardImage(id), alt: skin.card(id), popover: treacheryCardPopup(skin, id) }));
}

// A hero id of the event (`_heroId`, `_traitorToReplaceId`, ...): -1 for none.
export function idOrNull(v: unknown): number | null {
  return typeof v === "number" && v >= 0 ? v : null;
}

// MapPickComponent.PinActions: the reason the event isn't valid yet, Cancel and the confirm button.
export function PinActions({ label, validation, cancel, confirm }: { label: string; validation: Part[] | null; cancel: () => void; confirm: () => void }) {
  return (
    <>
      {validation && <div className="pin-error"><RichText parts={validation} /></div>}
      <div className="pin-actions">
        <button type="button" className="btn btn-secondary" onClick={cancel}>Cancel</button>
        <button type="button" className="btn btn-primary" disabled={validation !== null} onClick={confirm}>{label}</button>
      </div>
    </>
  );
}

// GenericComponents/MapForceStepper.razor: a small force counter for panels pinned on the map.
export function MapForceStepper({ value, onChange, min, max, faction, special }: {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  faction: Faction;
  special: boolean;
}) {
  const { skin } = useGame();
  const src = skin.url(skin.get(special ? "FactionSpecialForceImage_URL" : "FactionForceImage_URL", faction));
  const title = skin.get(special ? "SpecialForceName_STR" : "ForceName_STR", faction);
  const change = (delta: number) => {
    const next = Math.min(Math.max(value + delta, min), max);
    if (next !== value) onChange(next);
  };
  return (
    <div className="pin-step" title={title}>
      <span className="ic" style={{ background: skin.factionColor(faction) }}>
        {special && hasStarredSpecialForce(faction)
          ? <span className="force-token starred" style={{ ["--d" as string]: "16px" }}><img src={src} alt="" /></span>
          : <img src={src} alt="" />}
      </span>
      <button type="button" tabIndex={-1} aria-label="Fewer" disabled={value <= min} onClick={() => change(-1)}>−</button>
      <span className="val">{value}</span>
      <button type="button" tabIndex={-1} aria-label="More" disabled={value >= max} onClick={() => change(1)}>+</button>
    </div>
  );
}

// Location.Territory, from the static data.
export function useTerritoryOf(): (location: number) => number {
  const { skin } = useGame();
  return (l: number) => skin.locationInfo(l)?.territory ?? -1;
}
