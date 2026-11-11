// Pieces the revival forms (package 7d) share: the card every action form sits in, the force
// pickers, the yes/no radio, image choices for heroes and factions, and the draft hook.
import { useId, useState, type ReactNode } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { ActionOptions } from "../../generated/ActionOptions";
import type { Part } from "../../generated/Part";
import type { Faction } from "../../generated/enums";
import type { FactionOption } from "../../generated/FactionOption";
import type { HeroOption } from "../../generated/HeroOption";
import type { HeroRef } from "../../generated/HeroRef";
import type { GameView } from "../../generated/GameView";
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

// GameEventComponentLayout's collapse state, kept per component type for the session
// (SavedCollapsedStateValues).
const collapsedByType = new Map<string, boolean>();

// GenericComponents/GameEventComponentLayout.razor: a card with a collapsible header. Panels
// opened from a popover (the hand's card plays) start open and don't fade in.
export function GameEventLayout({ type, header, urgent = false, inPopover = false, children }: {
  type: string;
  header: ReactNode;
  urgent?: boolean;
  inPopover?: boolean;
  children: ReactNode;
}) {
  const [collapsed, setCollapsed] = useState(() => !inPopover && (collapsedByType.get(type) ?? false));
  const toggle = () => {
    collapsedByType.set(type, !collapsed);
    setCollapsed(!collapsed);
  };
  return (
    <div className={`card ${urgent ? "card-urgent" : ""} mb-2`} style={inPopover ? undefined : { animation: "fadeIn 1s" }}>
      <div className="card-header" style={{ cursor: "pointer" }} onClick={toggle}>
        <CollapseButton collapsed={collapsed} />
        {header}
      </div>
      {!collapsed && <div className="card-body">{children}</div>}
    </div>
  );
}

// GenericComponents/SelectForcesComponent.razor: a number picker with the faction's force or
// special force token as its symbol.
export function SelectForces({ value, onChange, min, max, faction, special, label = "" }: {
  value: number;
  onChange: (v: number) => void;
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
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolSrc={src}
                    symbolStarred={special && hasStarredSpecialForce(faction)} symbolFill={skin.factionColor(faction)} />
    </div>
  );
}

// GenericComponents/RadioComponent.razor with Values {false, true} and labels {"No", "Yes"}.
export function YesNo({ value, onChange, label }: { value: boolean; onChange: (v: boolean) => void; label: ReactNode }) {
  const id = useId();
  return (
    <>
      <label className="mb-0" htmlFor={id}>{label}</label>
      {[false, true].map((v) => (
        <div key={String(v)} className="form-check mb-0">
          <input className="form-check-input" style={{ cursor: "pointer" }} type="radio" id={`${id}-${v}`} value={String(v)}
                 checked={v === value} onChange={() => onChange(v)} />
          <label className="form-check-label" style={{ cursor: "pointer" }} htmlFor={`${id}-${v}`}>{v ? "Yes" : "No"}</label>
        </div>
      ))}
    </>
  );
}

// Skin.GetImageUrl(IHero).
export function heroImage(skin: Skin, h: HeroRef): string {
  if (h.t === "leader") return skin.personImage(h.id);
  if (h.t === "messiah") return skin.url(skin.str("Messiah_URL"));
  return skin.cardImage(h.id);
}

// Skin.Describe(IHero).
export function heroName(skin: Skin, h: HeroRef): string {
  if (h.t === "leader") return skin.person(h.id);
  if (h.t === "messiah") return skin.concept("Messiah");
  return skin.card(h.id);
}

// Skin.GetPopup(IHero, Game): a skilled leader shows its skill card.
function heroPopup(skin: Skin, view: GameView, h: HeroRef): string {
  if (h.t === "treacheryCard") return treacheryCardPopup(skin, h.id);
  if (h.t === "messiah") return messiahPopup(skin);
  const skill = view.players.flatMap((p) => p.skilledLeaders).find((s) => s.id === h.id)?.skill ?? null;
  return leaderPopup(skin, h.id, skill);
}

// The images of SelectFromImageComponent over heroes, valued by `_heroId`.
export function heroChoices(skin: Skin, view: GameView, heroes: HeroOption[]): ImageChoice<number>[] {
  return heroes.map((h) => ({ value: h.id, src: heroImage(skin, h.hero), alt: heroName(skin, h.hero), popover: heroPopup(skin, view, h.hero) }));
}

// The images of SelectFromImageComponent over factions, valued by the enum number. A faction
// has no rich popover, so Image.razor shows its name as a tooltip.
export function factionChoices(skin: Skin, factions: FactionOption[]): ImageChoice<number>[] {
  return factions.map((f) => ({ value: f.value, src: skin.factionImage(f.faction), alt: skin.faction(f.faction), popover: null }));
}

type Draft = GameEventDraft;
type WithDraft = Extract<ActionOptions, { draft: Record<string, unknown> }>;

// A revival form's state (formDraft.ts): the server's latest options and validation, whose
// `draft` is the event with the inputs as the component shows them (rust/view/src/schema/revival.rs).
export function useRevivalDraft<F extends WithDraft["form"]>(action: ActionView, form: F) {
  const { source, options, draft, validation, update, error, setError } = useServerDraft(action, form);
  const submit = async (event: Draft = draft) => setError(await source.submit(event));
  return { options, draft, validation, update, submit, error };
}

// The server's answer to a refused submission, under the form.
export function SubmitError({ error }: { error: string | null }) {
  return error ? <div className="text-danger small">{error}</div> : null;
}

// MapPickComponent.PinActions: the validation, Cancel and the confirm button, beside the pin.
export function PinActions({ label, validation, cancel, confirm }: { label: string; validation: Part[] | null; cancel: () => void; confirm: () => void }) {
  return (
    <div className="pin-body">
      {validation && <div className="pin-error"><RichText parts={validation} /></div>}
      <div className="pin-actions">
        <button type="button" className="btn btn-secondary" onClick={cancel}>Cancel</button>
        <button type="button" className="btn btn-primary" disabled={validation != null} onClick={confirm}>{label}</button>
      </div>
    </div>
  );
}
