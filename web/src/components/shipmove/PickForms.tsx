// The forms that pick one location or territory on the map (GenericComponents/MapPickComponent):
// every valid choice is outlined; clicking one pins it and shows a panel with the action's
// remaining inputs and its confirm button. Each form names its razor file.
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { ActionView } from "../../generated/ActionView";
import { AmbassadorValue, FactionValue, TerrorTypeValue, type Ambassador, type Faction, type TerrorType } from "../../generated/enums";
import { ambassadorPopup, terrorPopup, treacheryCardPopup, withSpiceIcons } from "../../popups";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { Image } from "../Image";
import { PlaceChip } from "../PlaceChip";
import { PopoverText } from "../PopoverText";
import { Select } from "../Select";
import { SelectFromImage } from "../SelectFromImage";
import { SimpleNumber } from "../SimpleNumber";
import { useMapPicker, type MapPick } from "../board/mapInteraction";
import { enumName, EventLayout, idOf, MapForceStepper, optionsOf, parseForceLocations, PinActions, PinChoice, useDraft, useForceNames, useTerritoryOf, type Draft, type ForceMap } from "./common";

function useFaction(): Faction {
  const { view } = useGame();
  return (view.viewer.faction ?? "None") as Faction;
}

// MapPickComponent: the draft plus the map picker. `toDraft` turns the pinned target (and the
// selected sources) into the draft's fields; a change of either is a new draft.
function usePick(action: ActionView, opts: {
  targets: number[] | ((selected: number[]) => number[]);
  sources?: number[];
  // Whether the pick is on; a function of the draft when an input of the form decides it.
  enabled?: boolean | ((draft: Draft) => boolean);
  byTerritory?: boolean;
  fields: () => Draft;
  toDraft: (target: number | null, selected: number[]) => Draft;
  pin: (target: number) => ReactNode;
  onPick?: (target: number | null, selected: number[]) => void;
}) {
  const faction = useFaction();
  const territoryOf = useTerritoryOf();
  const d = useDraft(action, () => ({ ...opts.fields(), ...opts.toDraft(null, []) }));
  const targets = opts.targets;
  const pick: MapPick = useMapPicker({
    sources: opts.sources,
    targets: typeof targets === "function" ? targets : () => targets,
    enabled: typeof opts.enabled === "function" ? opts.enabled(d.draft) : opts.enabled,
    byTerritory: opts.byTerritory,
    territoryOf,
    pin: opts.pin,
    faction,
  });
  const key = `${pick.target}|${pick.selected.join(",")}`;
  const first = useRef(true);
  useEffect(() => {
    if (first.current) { first.current = false; return; }
    opts.onPick?.(pick.target, pick.selected);
    d.set((cur) => ({ ...cur, ...opts.toDraft(pick.target, pick.selected) }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return { ...d, pick, territoryOf };
}

const hint = (text: string) => <div className="small">{text}</div>;

// PerformBeneGesseritPlacementComponent, PerformMoritaniSetupComponent,
// PerformHmsPlacementComponent, PerformHmsMovementComponent, KarmaHmsMovementComponent: a
// location and a confirm button.
export function LocationPickForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const faction = useFaction();
  const names = useForceNames(faction);
  const t = action.eventType;
  const o = optionsOf(action.options, "PerformBeneGesseritPlacement") ?? optionsOf(action.options, "PerformMoritaniSetup")
    ?? optionsOf(action.options, "PerformHmsPlacement") ?? optionsOf(action.options, "PerformHmsMovement") ?? optionsOf(action.options, "KarmaHmsMovement");
  const moves = optionsOf(action.options, "PerformHmsMovement") ?? optionsOf(action.options, "KarmaHmsMovement");
  const label = {
    PerformBeneGesseritPlacement: "Place advisor",
    PerformMoritaniSetup: "Place forces",
    PerformHmsPlacement: "Place stronghold",
    PerformHmsMovement: "Move",
    KarmaHmsMovement: `Use ${skin.enumValue("TreacheryCardType", "Karma")}`,
  }[t] ?? "Confirm";
  const passable = t === "PerformHmsMovement" || t === "KarmaHmsMovement";
  const p = usePick(action, {
    targets: o?.targets ?? [],
    fields: () => (passable ? { Passed: false } : {}),
    toDraft: (target) => ({ _targetId: target ?? -1 }),
    pin: () => <div className="pin-body"><PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label={label} /></div>,
  });
  const hms = <PlaceChip place={{ location: skin.statics.map.hiddenMobileStronghold }} />;
  const header: ReactNode = {
    PerformBeneGesseritPlacement: "Place one peaceful advisor in any territory of your choice. If you are alone in the territory, it is flipped to a fighter.",
    PerformMoritaniSetup: <>Place 6 <ForceToken faction={faction} size={20} title={names.force} /> in an empty territory of your choice.</>,
    PerformHmsPlacement: <>Position the {hms}</>,
    PerformHmsMovement: <>Move the {hms} one territory, or pass</>,
    KarmaHmsMovement: <>Use {skin.enumValue("TreacheryCardType", "Karma")} to move the {hms}?</>,
  }[t];
  const urgent = t === "PerformBeneGesseritPlacement" || t === "PerformMoritaniSetup";
  return (
    <EventLayout type={t} urgent={urgent} header={header}>
      {moves && <p className="text-info">You have {moves.movesLeft} move(s) left. </p>}
      {hint("Use the map to choose a location.")}
      {passable && <ButtonRow pass={() => void p.send({ Passed: true })} passText="Pass" />}
    </EventLayout>
  );
}

// FremenSentMonsterComponent: no panel; the status line says what to do, and the territory is
// picked on the map.
export function FremenSentMonsterForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "FremenSentMonster");
  const p = usePick(action, {
    targets: o?.targets ?? [],
    byTerritory: true,
    fields: () => ({}),
    toDraft: (target) => ({ _territoryId: target != null ? p.territoryOf(target) : -1 }),
    pin: (target) => (
      <div className="pin-body">
        <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label={`Appear in ${skin.territory(p.territoryOf(target))}`} />
      </div>
    ),
  });
  return null;
}

// BeneGesseritAccompaniesComponent.
export function BeneGesseritAccompaniesForm({ action }: { action: ActionView }) {
  const faction = useFaction();
  const names = useForceNames(faction);
  const p = usePick(action, {
    targets: optionsOf(action.options, "BeneGesseritAccompanies")?.targets ?? [],
    fields: () => ({ Accompanies: true, ExtraAdvisor: false }),
    toDraft: (target) => ({ _targetId: target ?? -1 }),
    pin: () => {
      const may = optionsOf(p.options, "BeneGesseritAccompanies")?.maySendExtraAdvisor ?? false;
      return (
        <div className="pin-body">
          {may && (
            <>
              <div className="pin-hint">How many <ForceToken faction={faction} special size={20} /> {names.special}?</div>
              <PinChoice values={[false, true]} value={p.draft.ExtraAdvisor === true} onChange={(v) => p.set({ ExtraAdvisor: v })} label={(v) => (v ? "2" : "1")} />
            </>
          )}
          <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label="Accompany" />
        </div>
      );
    },
  });
  return (
    <EventLayout type="BeneGesseritAccompanies" urgent>
      {hint("Use the map to choose a location.")}
      <ButtonRow pass={() => void p.send({ Accompanies: false })} passText="Don't accompany" />
    </EventLayout>
  );
}

function CardOrCunning({ card }: { card: number | null }) {
  const { skin } = useGame();
  return card != null ? <PopoverText popover={treacheryCardPopup(skin, card)}>{skin.card(card)}</PopoverText> : <>Nexus Cunning</>;
}

// ChoamMovePreventionComponent.
export function ChoamMovePreventionForm({ action }: { action: ActionView }) {
  const o = optionsOf(action.options, "ChoamMovePrevention");
  const p = usePick(action, {
    targets: o?.targets ?? [],
    byTerritory: true,
    fields: () => ({}),
    toDraft: (target) => ({ _territoryId: target != null ? p.territoryOf(target) : -1 }),
    pin: () => <div className="pin-body"><PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label="Prevent move" /></div>,
  });
  return (
    <EventLayout type="ChoamMovePrevention" header={<>Use <CardOrCunning card={o?.card ?? null} /> to prevent the current player from moving forces into a territory you occupy?</>}>
      {hint("Use the map to choose a territory.")}
    </EventLayout>
  );
}

// ChoamRemoveForceComponent. The pinned location decides whose forces are there, so the faction
// and force type follow it (OnPickChanged and SetTarget; the server makes those changes).
export function ChoamRemoveForceForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "ChoamRemoveForce");
  const p = usePick(action, {
    targets: o?.targets ?? [],
    fields: () => ({ Target: FactionValue.None, SpecialForce: false }),
    toDraft: (l) => ({ _locationId: l ?? -1 }),
    pin: () => {
      const target = enumName(FactionValue, p.draft.Target, "None");
      const special = p.draft.SpecialForce === true;
      const factions = optionsOf(p.options, "ChoamRemoveForce")?.factions ?? [];
      const choices = factions.find((f) => f.faction === target)?.specialChoices ?? [];
      const info = skin.statics.factions.find((f) => f.faction === target);
      return (
        <div className="pin-body">
          {factions.length > 1 && (
            <>
              <div className="pin-hint">Whose force?</div>
              <PinChoice values={factions.map((f) => f.faction)} value={target} onChange={(f) => p.set({ Target: FactionValue[f] })} label={(f) => skin.faction(f)} />
            </>
          )}
          {factions.length === 1 && <div className="pin-hint"><FactionChip faction={factions[0].faction} /></div>}
          {choices.length > 1 && info && (
            <PinChoice values={[false, true]} value={special} onChange={(v) => p.set({ SpecialForce: v })}
                       label={(v) => (v ? skin.get("SpecialForceName_STR", info.specialForce) : skin.get("ForceName_STR", info.force))} />
          )}
          <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label="Remove force" />
        </div>
      );
    },
  });
  return (
    <EventLayout type="ChoamRemoveForce" header={<>Use <CardOrCunning card={o?.card ?? null} /> to force a player to send 1 force to their reserves?</>}>
      {hint("Use the map to choose a location.")}
    </EventLayout>
  );
}

// DiscoveryEnteredComponent: the forces that may enter are the ones next to the pinned
// stronghold; it starts with all of them (OnPickChanged: a new pin clears the forces, and the
// server fills in every source).
export function DiscoveryEnteredForm({ action }: { action: ActionView }) {
  const faction = useFaction();
  const encode = (m: ForceMap) => m.map(([l, b]) => `${l}:${b.forces}|${b.specialForces}`).join(",");
  const p = usePick(action, {
    targets: optionsOf(action.options, "DiscoveryEntered")?.targets ?? [],
    fields: () => ({ _forceLocations: "", Passed: false }),
    toDraft: (target) => ({ _toId: target ?? -1, _forceLocations: "" }),
    pin: () => {
      const forces = parseForceLocations(p.draft._forceLocations);
      const sources = optionsOf(p.options, "DiscoveryEntered")?.sources ?? [];
      return (
        <div className="pin-body">
          {forces.map(([l, b]) => {
            const s = sources.find((x) => x.location === l);
            const amount = (v: number, special: boolean) => {
              const next = forces.map(([x, c]) => [x, x === l ? (special ? { ...c, specialForces: v } : { ...c, forces: v }) : c] as [number, typeof c]);
              p.set({ _forceLocations: encode(next) });
            };
            return (
              <div key={l} className="pin-row">
                {forces.length > 1 && <span className="pin-from"><PlaceChip place={{ location: l }} /></span>}
                {(s?.forces ?? 0) > 0 && <MapForceStepper value={b.forces} onChange={(v) => amount(v, false)} max={s!.forces} faction={faction} />}
                {(s?.specialForces ?? 0) > 0 && <MapForceStepper value={b.specialForces} onChange={(v) => amount(v, true)} max={s!.specialForces} faction={faction} special />}
              </div>
            );
          })}
          <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label="Enter" />
        </div>
      );
    },
  });
  return (
    <EventLayout type="DiscoveryEntered" header="Enter a discovered stronghold?">
      {hint("Use the map to choose a location.")}
      <ButtonRow pass={() => void p.send({ Passed: true })} passText="Don't enter" />
    </EventLayout>
  );
}

// AmbassadorPlacedComponent.
export function AmbassadorPlacedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "AmbassadorPlaced");
  const p = usePick(action, {
    targets: o?.targets ?? [],
    byTerritory: true,
    fields: () => ({ Ambassador: AmbassadorValue[(o?.ambassadors[0] ?? "None") as Ambassador] }),
    toDraft: (target) => ({ _strongholdId: target != null ? p.territoryOf(target) : -1 }),
    pin: () => (
      <div className="pin-body">
        <div className="pin-row">
          {ambassador !== "None" && <Image src={skin.url(skin.get("AmbassadorImage_URL", ambassador))} alt={skin.get("AmbassadorName_STR", ambassador)} width={28} popover={ambassadorPopup(skin, ambassador)} />}
          <span>{ambassador !== "None" ? skin.get("AmbassadorName_STR", ambassador) : ""} ambassador</span>
        </div>
        <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label="Place ambassador" />
      </div>
    ),
  });
  const ambassador = enumName(AmbassadorValue, p.draft.Ambassador, "None");
  const choose = (a: Ambassador | null) => p.set({ Ambassador: AmbassadorValue[a ?? "None"] });
  return (
    <EventLayout type="AmbassadorPlaced" header="You may now place an Ambassador Token in a Stronghold">
      <SelectFromImage label="Which ambassador?" value={ambassador === "None" ? null : ambassador} onChange={choose}
                       values={(o?.ambassadors ?? []).map((a) => ({ value: a as Ambassador, src: skin.url(skin.get("AmbassadorImage_URL", a)), alt: skin.get("AmbassadorName_STR", a), popover: ambassadorPopup(skin, a as Ambassador) }))} />
      <div className="small mt-1">Use the map to choose a stronghold.</div>
      <div>Cost: <SimpleNumber size={20}>{o?.cost ?? 0}</SimpleNumber></div>
    </EventLayout>
  );
}

// TerrorPlantedComponent. Removing a token names no stronghold, so the map is left alone.
export function TerrorPlantedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "TerrorPlanted");
  const removing = o?.mayRemove ?? false;
  const describe = (t: TerrorType) => <span dangerouslySetInnerHTML={{ __html: withSpiceIcons(skin, skin.get("TerrorTokenDescription_STR", t)) }} />;
  const choose = (t: TerrorType) => p.set({ Type: TerrorTypeValue[t] });
  const p = usePick(action, {
    targets: o?.targets ?? [],
    enabled: !removing,
    byTerritory: true,
    fields: () => ({ Type: TerrorTypeValue[((removing ? o?.removeTypes[0] : undefined) ?? "None") as TerrorType], Passed: false }),
    toDraft: (target) => ({ _territoryId: target != null ? p.territoryOf(target) : -1 }),
    pin: () => {
      // PinContent's fallback to the first type is the server's.
      const types = (o?.placeTypes ?? []) as TerrorType[];
      const current = enumName(TerrorTypeValue, p.draft.Type, "None");
      return (
        <div className="pin-body">
          <PinChoice wrap values={types} value={current} onChange={choose} label={(t) => skin.get("TerrorTokenName_STR", t)} />
          <div className="pin-text">{describe(current)}</div>
          <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm}
                      label={(o?.unplaced ?? []).includes(current) ? "Place token" : "Move token"} />
        </div>
      );
    },
  });
  const type = enumName(TerrorTypeValue, p.draft.Type, "None");
  const removeTypes = (o?.removeTypes ?? []) as TerrorType[];
  return (
    <EventLayout type="TerrorPlanted" header={removing ? "You may now remove Terror" : "You may now place Terror"}>
      {removing ? (
        <>
          <Select required label="Token" options={removeTypes.map((t) => ({ value: t, label: skin.get("TerrorTokenName_STR", t) }))} value={type} onChange={choose} />
          <div className="text-info">{describe(type)}</div>
          <ButtonRow pass={() => void p.send({ Passed: true })} confirm={p.confirm} confirmText="Remove token" confirmError={p.validation} />
        </>
      ) : (
        <>
          {hint("Use the map to choose a stronghold.")}
          <ButtonRow pass={() => void p.send({ Passed: true })} passText="Pass" />
        </>
      )}
    </EventLayout>
  );
}

// GenericComponents/RadioComponent.razor.
export function Radio<T>({ label, values, labels, value, onChange }: { label?: ReactNode; values: T[]; labels: string[]; value: T; onChange: (v: T) => void }) {
  const { skin } = useGame();
  const [id] = useState(() => `radio${Math.random().toString(36).slice(2, 8)}`);
  return (
    <>
      {label != null && <label className="mb-0" htmlFor={id}>{label}</label>}
      {values.map((v, i) => (
        <div key={i} className="form-check mb-0">
          <input className="form-check-input" style={{ cursor: "pointer" }} type="radio" id={`${id}-${i}`} value={i} onChange={() => onChange(v)} checked={v === value} />
          <label className="form-check-label" style={{ cursor: "pointer" }} htmlFor={`${id}-${i}`} dangerouslySetInnerHTML={{ __html: withSpiceIcons(skin, labels[i]) }} />
        </div>
      ))}
    </>
  );
}

// TerrorRevealedComponent.
export function TerrorRevealedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const faction = useFaction();
  const o = optionsOf(action.options, "TerrorRevealed");
  const types = (o?.types ?? []) as TerrorType[];
  const typeOf = (d: Draft) => enumName(TerrorTypeValue, d.Type, "None");
  const sneakPossible = (o?.sneakAttackTargets.length ?? 0) > 0;
  const p = usePick(action, {
    targets: o?.sneakAttackTargets ?? [],
    enabled: (d) => typeOf(d) === "SneakAttack" && sneakPossible,
    fields: () => ({ AllianceOffered: false, Type: TerrorTypeValue[types[0] ?? "None"], ForcesInSneakAttack: 0, RobberyTakesCard: false, _cardToGiveInSabotageId: -1, Passed: false }),
    toDraft: (target) => ({ _sneakAttackTo: target ?? -1 }),
    // OnPickChanged: a pinned target starts with the most forces.
    onPick: (target) => {
      if (target != null && p.get().ForcesInSneakAttack === 0) p.set({ ForcesInSneakAttack: o?.maxSneakAttackForces ?? 0 });
    },
    pin: () => (
      <div className="pin-body">
        <div className="pin-row">
          <MapForceStepper value={(p.draft.ForcesInSneakAttack as number) ?? 0} onChange={(v) => p.set({ ForcesInSneakAttack: v })} max={o?.maxSneakAttackForces ?? 0} faction={faction} />
        </div>
        <PinActions validation={p.validation} cancel={p.pick.clear} confirm={p.confirm} label="Sneak attack" />
      </div>
    ),
  });
  // The markup drops the robbery choice, the card and the forces of the other kinds; the
  // server does the same to the draft.
  const chooseType = (t: TerrorType) => p.set({ Type: TerrorTypeValue[t] });
  const type = typeOf(p.draft);
  const sneak = type === "SneakAttack";
  const robberyTakesCard = p.draft.RobberyTakesCard === true;
  const sabotageCard = idOf(p.draft._cardToGiveInSabotageId);
  const territory = o?.territory;
  const termName = (t: TerrorType) => skin.get("TerrorTokenName_STR", t);
  return (
    <EventLayout type="TerrorRevealed" urgent header={<>Resort to Terror against {o && <FactionChip faction={o.victim} />} in {territory != null && <PlaceChip place={{ territory }} />}?</>}>
      {types.length > 1 ? (
        <Select required label="Which kind of terror to use?" options={types.map((t) => ({ value: t, label: termName(t) }))} value={type} onChange={chooseType} />
      ) : (
        <div>Use <PopoverText popover={terrorPopup(skin, type)}>{termName(type)}</PopoverText>?</div>
      )}
      {type === "Robbery" && (
        <Radio label="Which kind of robbery?" values={[false, true]} labels={["Steal half their spice (rounded up)", "Draw a Treachery Card"]} value={robberyTakesCard}
               onChange={(v) => p.set({ RobberyTakesCard: v })} />
      )}
      {type === "Sabotage" && (
        <SelectFromImage label="You may select one of your treachery cards to give to them:" required={false} imageWidth={80} value={sabotageCard}
                         onChange={(c) => p.set({ _cardToGiveInSabotageId: c ?? -1 })}
                         values={(o?.cards ?? []).map((c) => ({ value: c, src: skin.cardImage(c), alt: skin.card(c), popover: treacheryCardPopup(skin, c) }))} />
      )}
      {sneak && (sneakPossible ? hint("Use the map to choose a location.") : <div className="alert alert-danger mt-2">You cannot send forces there.</div>)}
      {o?.mayOfferAlliance && <div>You may also offer them an alliance as an alternative to violence.</div>}
      {o?.mayPass ? (
        <ButtonRow confirm={p.confirm} confirmError={p.validation} pass={() => void p.send({ Passed: true })}
                   other={o.mayOfferAlliance ? () => void p.send({ AllianceOffered: true }) : undefined} otherText="Offer Alliance" />
      ) : (
        <ButtonRow confirm={p.confirm} confirmError={p.validation} />
      )}
    </EventLayout>
  );
}

