// The ship-and-move forms without a map pick of their own, and the two with special map use
// (AmbassadorActivated moves or ships forces on the map; PerformFremenSetup stages forces).
import { useEffect, useRef, useState } from "react";
import type { ActionView } from "../../generated/ActionView";
import { AmbassadorValue, FactionValue, type Ambassador, type DiscoveryToken, type Faction } from "../../generated/enums";
import { ambassadorPopup, discoveryTokenPopup, leaderPopup, treacheryCardPopup, withSpiceIcons } from "../../popups";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { Checkbox } from "../Checkbox";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { Image } from "../Image";
import { NumberPicker } from "../NumberPicker";
import { PlaceChip } from "../PlaceChip";
import { PopoverText } from "../PopoverText";
import { Select } from "../Select";
import { SelectFromImage } from "../SelectFromImage";
import { SpiceComponent } from "../SpiceComponent";
import { isPlainClick, mapInteraction, onLocationClick, useMapInteraction, useMapPicker } from "../board/mapInteraction";
import { enumName, EventLayout, forceLocationsString, idOf, MapForceStepper, optionsOf, parseForceLocations, PinActions, useDraft, useForceNames, useTerritoryOf, Validation, type ForceMap } from "./common";
import { Radio } from "./PickForms";

function useFaction(): Faction {
  const { view } = useGame();
  return (view.viewer.faction ?? "None") as Faction;
}

// BeneGesseritFlipComponent.
export function BeneGesseritFlipForm({ action }: { action: ActionView }) {
  const o = optionsOf(action.options, "BeneGesseritFlip");
  const { validation, send } = useDraft(action, () => ({ AsAdvisors: false }));
  return (
    <EventLayout type="BeneGesseritFlip" urgent header={<>Choose to be fighters or advisors in {o && <PlaceChip place={{ territory: o.territory }} />}.</>}>
      <ButtonRow confirm={() => void send({ AsAdvisors: false })} confirmText="Fighters" confirmError={validation}
                 other={() => void send({ AsAdvisors: true })} otherText="Advisors" />
    </EventLayout>
  );
}

// FlightUsedComponent.
export function FlightUsedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const { validation, send } = useDraft(action, () => ({ MoveThreeTerritories: true }));
  return (
    <EventLayout type="FlightUsed" header={`Use ${skin.enumValue("TreacheryCardType", "Flight")} to be able to move 3 territories once OR move 2 different force groups?`}>
      <ButtonRow confirm={() => void send({ MoveThreeTerritories: true })} confirmText="Move 3 territories" confirmError={validation}
                 other={() => void send({ MoveThreeTerritories: false })} otherText="Move 2 different groups" />
    </EventLayout>
  );
}

// TestingStationUsedComponent.
export function TestingStationUsedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const { draft, validation, set, confirm } = useDraft(action, () => ({ ValueAdded: -1 }));
  const token: DiscoveryToken = "TestingStation";
  return (
    <EventLayout type="TestingStationUsed" header={<>Use the <PopoverText popover={discoveryTokenPopup(skin, token)}>{skin.get("DiscoveryTokenName_STR", token)}</PopoverText>?</>}>
      <Radio values={[-1, 1]} labels={["Subtract one from storm movement", "Add one to storm movement"]} value={draft.ValueAdded as number} onChange={(v) => set({ ValueAdded: v })} />
      <ButtonRow confirm={confirm} confirmError={validation} />
    </EventLayout>
  );
}

// KarmaShipmentPreventionComponent.
export function KarmaShipmentPreventionForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "KarmaShipmentPrevention");
  const first = (o?.targets[0] ?? "None") as Faction;
  const { draft, validation, set, confirm } = useDraft(action, () => ({ Target: FactionValue[first] }));
  const target = enumName(FactionValue, draft.Target, "None");
  return (
    <EventLayout type="KarmaShipmentPrevention" header={`Use ${skin.enumValue("TreacheryCardType", "Karma")} to prevent shipment`}>
      <SelectFromImage label="Target" imageWidth={40} value={target === "None" ? null : target} onChange={(f) => set({ Target: FactionValue[f ?? "None"] })}
                       values={(o?.targets ?? []).map((f) => ({ value: f as Faction, src: skin.factionImage(f as Faction), alt: skin.faction(f as Faction), popover: null }))} />
      <ButtonRow confirm={confirm} confirmText="Prevent Shipment" confirmError={validation} />
    </EventLayout>
  );
}

// SetShipmentPermissionComponent.
export function SetShipmentPermissionForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "SetShipmentPermission");
  const [targets, setTargets] = useState<Faction[]>([]);
  const [cross, setCross] = useState(false);
  const [homeworld, setHomeworld] = useState(false);
  const [discount, setDiscount] = useState(false);
  const permission = (c: boolean, h: boolean, d: boolean) => (c ? 1 : 0) | (h ? 2 : 0) | (d ? 4 : 0);
  const { validation, set, confirm, send } = useDraft(action, () => ({ Factions: [], Permission: 0 }));
  const update = (t: Faction[], c: boolean, h: boolean, d: boolean) => {
    setTargets(t); setCross(c); setHomeworld(h); setDiscount(d);
    set({ Factions: t.map((f) => FactionValue[f]), Permission: permission(c, h, d) });
  };
  const yes = (b: boolean) => (b ? "Yes" : "No");
  return (
    <EventLayout type="SetShipmentPermission" header="Manage shipment permissions">
      {(o?.permissions.length ?? 0) > 0 && (
        <table className="table table-sm">
          <thead>
            <tr className="table-dark">
              <th scope="col">Faction</th>
              <th scope="col">Cross planet</th>
              <th scope="col">To (own) homeworld</th>
              <th scope="col">At half price</th>
              <th scope="col">Action</th>
            </tr>
          </thead>
          <tbody>
            {o!.permissions.map((row) => (
              <tr key={row.faction} className="table-dark">
                <td><img width={24} src={skin.factionImage(row.faction)} /></td>
                <td>{yes(row.cross)}</td>
                <td>{yes(row.toHomeworld)}</td>
                <td>{yes(row.guildRate)}</td>
                <td><button className="btn btn-primary btn-sm" onClick={() => void send({ Factions: [FactionValue[row.faction]], Permission: 0 })}>Revoke</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <div className="mt-2">
        {/* SelectFromImageComponent with a Selected list: each click toggles a faction. */}
        <label className="d-block mt-1 mb-0">Set permissions for:</label>
        <div className="img-choices">
          {(o?.targets ?? []).map((f) => {
            const faction = f as Faction;
            const on = targets.includes(faction);
            return (
              <Image key={f} src={skin.factionImage(faction)} alt={skin.faction(faction)} width={40} pointer highlightOnHover darkened={!on}
                     onClick={() => update(on ? targets.filter((x) => x !== faction) : [...targets, faction], cross, homeworld, discount)} />
            );
          })}
        </div>
      </div>
      <div className="mt-1">
        <Checkbox value={cross} onChange={(v) => update(targets, v, homeworld, discount)}>Allow cross planet shipments</Checkbox>
        <Checkbox value={homeworld} onChange={(v) => update(targets, cross, v, discount)}>Allow shipment to (own) homeworld</Checkbox>
        <Checkbox value={discount} onChange={(v) => update(targets, cross, homeworld, v)}>May ship at half price</Checkbox>
      </div>
      <ButtonRow confirm={confirm} confirmError={validation} />
    </EventLayout>
  );
}

// DiscoveryRevealedComponent.
export function DiscoveryRevealedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "DiscoveryRevealed");
  const locations = o?.locations ?? [];
  const [selected, setSelected] = useState(locations[0]?.location ?? -1);
  const { validation, set, confirm, send } = useDraft(action, () => ({ _locationId: locations[0]?.location ?? -1, Passed: false }));
  const current = locations.find((l) => l.location === selected) ?? locations[0];
  const token = current?.token as DiscoveryToken | undefined;
  return (
    <EventLayout type="DiscoveryRevealed" header="Reveal a Discovery?">
      {locations.length > 1 && (
        <Select required label="Where?" options={locations.map((l) => ({ value: l.location, label: skin.location(l.location) }))} value={selected}
                onChange={(v) => { setSelected(v); set({ _locationId: v }); }} />
      )}
      {current && token && (
        <>
          <div>Reveal the <PopoverText popover={discoveryTokenPopup(skin, token)}>{skin.get("DiscoveryTokenName_STR", token)}</PopoverText> in <PlaceChip place={{ territory: current.territory }} />?</div>
          <div className="label-info" dangerouslySetInnerHTML={{ __html: withSpiceIcons(skin, skin.get("DiscoveryTokenDescription_STR", token)) }} />
        </>
      )}
      <ButtonRow confirm={confirm} confirmError={validation} pass={() => void send({ Passed: true })} />
    </EventLayout>
  );
}

// PerformFremenSetupComponent (StagedPlacementComponent): click a sector for a force,
// shift-click for a Fedaykin, right-click to take one back. The list below the instructions is
// a fallback for people who cannot right-click.
export function PerformFremenSetupForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const faction = useFaction();
  const names = useForceNames(faction);
  const o = optionsOf(action.options, "PerformFremenSetup");
  const [forces, setForces] = useState<ForceMap>([]);
  const history = useRef<Array<[number, boolean]>>([]);
  const st = useRef(forces);
  st.current = forces;
  const self = useRef<object>({});
  const { validation, set, confirm } = useDraft(action, () => ({ _forceLocations: "" }));
  useMapInteraction();
  const maxTotal = o?.maxTotal ?? 10;
  const staged = (special: boolean) => st.current.reduce((n, [, b]) => n + (special ? b.specialForces : b.forces), 0);
  const room = (special: boolean) => {
    const reserve = special ? (o?.specialForcesInReserve ?? 0) - staged(true) : (o?.forcesInReserve ?? 0) - staged(false);
    const total = maxTotal - staged(false) - staged(true);
    return Math.max(0, Math.min(reserve, total));
  };
  const commit = (next: ForceMap) => {
    st.current = next;
    setForces(next);
    set({ _forceLocations: forceLocationsString(next) });
  };
  const add = (l: number, special: boolean) => {
    if (!(o?.targets ?? []).includes(l) || (special && !o?.mayStageSpecial) || room(special) === 0) return;
    const next: ForceMap = st.current.map(([x, b]) => [x, { ...b }]);
    let entry = next.find(([x]) => x === l);
    if (!entry) { entry = [l, { forces: 0, specialForces: 0 }]; next.push(entry); }
    if (special) entry[1].specialForces++; else entry[1].forces++;
    history.current.push([l, special]);
    commit(next);
  };
  const remove = (l: number, special?: boolean) => {
    const h = history.current;
    let index = -1;
    for (let i = h.length - 1; i >= 0; i--) if (h[i][0] === l && (special === undefined || h[i][1] === special)) { index = i; break; }
    if (index < 0) return;
    const wasSpecial = h[index][1];
    h.splice(index, 1);
    const next: ForceMap = st.current.map(([x, b]) => [x, x === l ? (wasSpecial ? { ...b, specialForces: b.specialForces - 1 } : { ...b, forces: b.forces - 1 }) : b]);
    commit(next.filter(([, b]) => b.forces + b.specialForces > 0));
  };
  useEffect(() => onLocationClick((c) => {
    if (!mapInteraction.isOwnedBy(self.current)) return;
    if (c.right) remove(c.location);
    else if (c.shift && !c.ctrl && !c.alt) add(c.location, true);
    else if (isPlainClick(c)) add(c.location, false);
  }));
  // PushToMap after every render while owning the map, ClaimIfFree.
  useEffect(() => {
    const map = mapInteraction;
    if (!map.isOwnedBy(self.current)) {
      if (!map.isFree) return;
      map.claim(self.current);
    }
    map.faction = faction;
    map.selectable.clear();
    map.selected.length = 0;
    map.target = null;
    map.pin = null;
    map.showCursorArrow = false;
    map.targets.clear();
    (o?.targets ?? []).forEach((l) => map.targets.add(l));
    map.staged.clear();
    for (const [l, b] of forces) map.staged.set(l, { faction, forces: b.forces, specialForces: b.specialForces });
    map.refresh();
  });
  useEffect(() => mapInteraction.subscribe(() => { if (mapInteraction.isFree) mapInteraction.claim(self.current); }), []);
  useEffect(() => () => mapInteraction.release(self.current), []);

  const remaining = maxTotal - staged(false) - staged(true);
  const shown = [...forces].filter(([, b]) => b.forces + b.specialForces > 0).sort(([a], [b]) => skin.location(a).localeCompare(skin.location(b)));
  const step = (l: number, value: number, special: boolean) => {
    const b = forces.find(([x]) => x === l)?.[1];
    const current = special ? b?.specialForces ?? 0 : b?.forces ?? 0;
    if (value > current) add(l, special);
    else if (value < current) remove(l, special);
  };
  const places = o?.places ?? [];
  return (
    <EventLayout type="PerformFremenSetup" urgent header={remaining > 0
      ? <>Distribute {remaining} forces on <PartsPlace parts={places[0]} />, <PartsPlace parts={places[1]} />, and <PartsPlace parts={places[2]} />.</>
      : <>All forces placed.</>}>
      <div className="small">Use the map to place forces.</div>
      <div className="small text-2">
        <div><ClickHint button="Lmb" /> place <ForceToken faction={faction} size={18} imgClass="setup-hint-force" title={names.force} /></div>
        {o?.mayStageSpecial && <div><ClickHint button="Lmb" shift /> place <ForceToken faction={faction} special size={18} imgClass="setup-hint-force" title={names.special} /></div>}
        <div><ClickHint button="Rmb" /> remove force</div>
      </div>
      {shown.length > 0 && (
        <div className="mt-2">
          {shown.map(([l, b]) => (
            <div key={l} className="d-flex align-items-center gap-1 mt-1 small">
              <span className="flex-grow-1"><PlaceChip place={{ location: l }} /></span>
              <MapForceStepper value={b.forces} onChange={(v) => step(l, v, false)} max={b.forces + room(false)} faction={faction} label={names.force} />
              {o?.mayStageSpecial && <MapForceStepper value={b.specialForces} onChange={(v) => step(l, v, true)} max={b.specialForces + room(true)} faction={faction} special label={names.special} />}
            </div>
          ))}
        </div>
      )}
      <Validation parts={validation} className="small text-danger mt-2" />
      <ButtonRow confirm={confirm} confirmText="Confirm" confirmError={validation} />
    </EventLayout>
  );
}

// A place the server rendered as one rich text part.
function PartsPlace({ parts }: { parts: import("../../generated/Part").Part[] | undefined }) {
  const p = parts?.[0];
  if (!p) return null;
  if (p.t === "location") return <PlaceChip place={{ location: p.id }} />;
  if (p.t === "territory") return <PlaceChip place={{ territory: p.id }} />;
  return null;
}

// GenericComponents/ClickHintComponent.razor: a key to hold, then a mouse with the button to press.
const MOUSE = "M181.563,0C120.762,0,59.215,30.525,59.215,88.873V237.5c0,65.658,53.412,119.071,119.071,119.071 c65.658,0,119.07-53.413,119.07-119.071V88.873C297.356,27.809,237.336,0,181.563,0z M274.945,237.5 c0,53.303-43.362,96.657-96.659,96.657c-53.299,0-96.657-43.354-96.657-96.657v-69.513c20.014,6.055,57.685,15.215,102.221,15.215 c28.515,0,59.831-3.809,91.095-14.567V237.5z M274.945,144.794c-81.683,31.233-168.353,7.716-193.316-0.364V88.873 c0-43.168,51.489-66.46,99.934-66.46c46.481,0,93.382,20.547,93.382,66.46V144.794z M190.893,48.389v81.248 c0,6.187-5.023,11.208-11.206,11.208c-6.185,0-11.207-5.021-11.207-11.208V48.389c0-6.186,5.021-11.207,11.207-11.207 C185.869,37.182,190.893,42.203,190.893,48.389z ";

function ClickHint({ button, shift = false }: { button: "Lmb" | "Rmb"; shift?: boolean }) {
  const left = button === "Lmb";
  return (
    <>
      {shift && <span><kbd className="small">shift</kbd>+</span>}
      <svg x="0" y="0" fill="currentColor" width="16" height="16" viewBox="0 0 356.572 356.572" style={{ margin: "-1px 0 0 0" }}>
        <title>{left ? "click map to select" : "right-click map to select"}</title>
        <path d={MOUSE + (left ? "M154.938,40.068V143.73c-15.879,2.802-62.566-10.271-62.566-10.271 C80.233,41.004,154.938,40.068,154.938,40.068z" : "M264.272,130.378c0,0-46.687,13.072-62.566,10.271V36.988 C201.706,36.988,276.412,37.923,264.272,130.378z")} />
      </svg>
    </>
  );
}

// AmbassadorActivatedComponent. The Fremen ambassador moves forces like a regular move (sources
// first, then the destination); the Guild one ships from reserves (destination only).
export function AmbassadorActivatedForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const faction = useFaction();
  const territoryOf = useTerritoryOf();
  const o0 = optionsOf(action.options, "AmbassadorActivated");
  const initialActual = (o0?.ambassador === "BeneGesserit" ? o0.beneGesseritAmbassadors[0] ?? "None" : o0?.ambassador ?? "None") as Ambassador;
  const initialIxian = initialActual === "Ixian" ? o0?.ixianCards[0] ?? null : null;
  const d = useDraft(action, () => ({
    BeneGesseritSelectedAmbassador: AmbassadorValue[initialActual], EcazOfferAlliance: false, EcazGiveVidalToAlly: false, EcazTakeVidal: false,
    GuildForceAmount: 0, TleilaxuAmountOfForces: 0, TleilaxuAssignSkill: false, _choamCardIds: "", _fremenOrGuildToId: -1,
    _fremenForceLocations: "", _ixianCardId: initialIxian ?? -1, _tleilaxuHeroId: -1, Passed: false,
  }));
  const o = optionsOf(d.options, "AmbassadorActivated") ?? o0;
  // The component's fields, from the draft. The server makes the markup's adjustments: the
  // shown ambassador or the first valid one, Ecaz checkboxes that no longer apply, Tleilaxu
  // forces without room, a skill for a hero that can't take one, the Guild amount without
  // targets, the first Ixian card.
  const actual = enumName(AmbassadorValue, d.draft.BeneGesseritSelectedAmbassador, "None");
  const choamCards = typeof d.draft._choamCardIds === "string" && d.draft._choamCardIds !== "" ? d.draft._choamCardIds.split(";").map(Number) : [];
  const takeVidal = d.draft.EcazTakeVidal === true;
  const offerAlliance = d.draft.EcazOfferAlliance === true;
  const giveVidal = d.draft.EcazGiveVidalToAlly === true;
  const fremenForces = parseForceLocations(d.draft._fremenForceLocations);
  const ixianCard = idOf(d.draft._ixianCardId);
  const guildAmount = typeof d.draft.GuildForceAmount === "number" ? d.draft.GuildForceAmount : 0;
  const tleilaxuForces = typeof d.draft.TleilaxuAmountOfForces === "number" ? d.draft.TleilaxuAmountOfForces : 0;
  const tleilaxuHero = idOf(d.draft._tleilaxuHeroId);
  const assignSkill = d.draft.TleilaxuAssignSkill === true;
  const victim = (o?.victim ?? "None") as Faction;
  const movesForces = actual === "Fremen";
  const shipsForces = actual === "Guild";
  const fremenSources = (o?.fremenSources ?? []).map((s) => s.location);
  const guildTargets = o?.guildTargets ?? [];
  const pick = useMapPicker({
    sources: movesForces ? fremenSources : [],
    targets: (selected) => (movesForces ? (selected.length > 0 ? o?.fremenTargets ?? [] : []) : shipsForces ? guildTargets : []),
    enabled: (movesForces && fremenSources.length > 0) || (shipsForces && guildTargets.length > 0),
    territoryOf,
    faction,
    pin: () => (
      <div className="pin-body">
        {movesForces ? (
          <>
            {fremenForces.map(([l, b]) => {
              const s = o?.fremenSources.find((x) => x.location === l);
              return (
                <div key={l} className="pin-row">
                  {fremenForces.length > 1 && <span className="pin-from"><PlaceChip place={{ location: l }} /></span>}
                  {(s?.forces ?? 0) > 0 && <MapForceStepper value={b.forces} onChange={(v) => setFremen(l, v, false)} max={s!.forces} faction={faction} />}
                  {(s?.specialForces ?? 0) > 0 && <MapForceStepper value={b.specialForces} onChange={(v) => setFremen(l, v, true)} max={s!.specialForces} faction={faction} special />}
                </div>
              );
            })}
            <PinActions validation={d.validation} cancel={pick.clear} confirm={d.confirm} label="Move" />
          </>
        ) : (
          <>
            <div className="pin-row">
              <MapForceStepper value={guildAmount} onChange={(v) => d.set({ GuildForceAmount: v })} max={o?.guildMaxForces ?? 0} faction={faction} />
            </div>
            <PinActions validation={d.validation} cancel={pick.clear} confirm={d.confirm} label="Send" />
          </>
        )}
      </div>
    ),
  });
  const setFremen = (l: number, v: number, special: boolean) => {
    const next: ForceMap = fremenForces.map(([x, b]) => [x, x === l ? (special ? { ...b, specialForces: v } : { ...b, forces: v }) : b]);
    d.set({ _fremenForceLocations: forceLocationsString(next) });
  };
  // OnPickChanged: moving starts with everything in the clicked sectors; shipping with one force.
  const key = `${pick.target}|${pick.selected.join(",")}`;
  useEffect(() => {
    const kept = fremenForces.filter(([l]) => pick.selected.includes(l));
    for (const l of pick.selected) {
      if (!kept.some(([x]) => x === l)) {
        const s = o?.fremenSources.find((x) => x.location === l);
        kept.push([l, { forces: s?.forces ?? 0, specialForces: s?.specialForces ?? 0 }]);
      }
    }
    let guild = guildAmount;
    if (shipsForces && pick.target != null && guild === 0) guild = Math.min(1, o?.guildMaxForces ?? 0);
    d.set({ _fremenOrGuildToId: pick.target ?? -1, _fremenForceLocations: forceLocationsString(kept), GuildForceAmount: guild });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const chooseActual = (a: Ambassador | null) => {
    d.set({ BeneGesseritSelectedAmbassador: AmbassadorValue[(a ?? "None") as Ambassador] });
  };
  const cardChoice = (c: number) => ({ value: c, src: skin.cardImage(c), alt: skin.card(c), popover: treacheryCardPopup(skin, c) });
  const forceToken = <ForceToken faction={faction} size={20} />;
  const vidal = o?.vidal != null ? <PopoverText popover={leaderPopup(skin, o.vidal, null)}>{skin.person(o.vidal)}</PopoverText> : null;

  let body: React.ReactNode = null;
  switch (actual) {
    case "Atreides": body = <div>See the <FactionChip faction={victim} /> hand?</div>; break;
    case "Choam":
      body = (
        <>
          <div>Discard cards from your hand for <SpiceComponent amount="3" /> each?</div>
          <label className="d-block mt-1 mb-0">Select the cards you wish to discard for <SpiceComponent amount="3" /> each</label>
          <div className="img-choices">
            {(o?.choamCards ?? []).map((c) => {
              const on = choamCards.includes(c);
              return (
                <Image key={c} src={skin.cardImage(c)} alt={skin.card(c)} width={80} popover={treacheryCardPopup(skin, c)} pointer highlightOnHover darkened={!on}
                       onClick={() => { const next = on ? choamCards.filter((x) => x !== c) : [...choamCards, c]; d.set({ _choamCardIds: next.join(";") }); }} />
              );
            })}
          </div>
        </>
      );
      break;
    case "Ecaz":
      body = (
        <>
          {o?.vidalCanBeTaken && <Checkbox value={takeVidal} onChange={(v) => d.set({ EcazTakeVidal: v })}>Take {vidal}</Checkbox>}
          {!takeVidal && o?.allianceCanBeOffered && (
            <>
              <Checkbox value={offerAlliance} onChange={(v) => d.set({ EcazOfferAlliance: v })}>Offer an alliance to <FactionChip faction={victim} /></Checkbox>
              {offerAlliance && o.vidalCanBeOfferedToNewAlly && (
                <Checkbox value={giveVidal} onChange={(v) => d.set({ EcazGiveVidalToAlly: v })}>Give {vidal} to <FactionChip faction={victim} /> if they agree to ally</Checkbox>
              )}
            </>
          )}
        </>
      );
      break;
    case "Emperor": body = <div>Receive <SpiceComponent amount="5" />?</div>; break;
    case "Fremen":
      body = (
        <>
          <div>Move a group of your forces?</div>
          {fremenSources.length > 0
            ? <div className="small">Use the map to move forces: click the sector to move from, then where to move to.</div>
            : <div>You don't have any forces to move.</div>}
        </>
      );
      break;
    case "Harkonnen":
      body = victim === "Harkonnen" ? <div>See one of the <FactionChip faction={victim} /> traitors?</div>
        : victim === "Tleilaxu" ? <div>See one of the <FactionChip faction={victim} /> unrevealed Face Dancers?</div>
        : <div>See the <FactionChip faction={victim} /> traitor?</div>;
      break;
    case "Ixian":
      body = (
        <>
          <div>Discard a card from your hand to draw a new one?</div>
          <SelectFromImage label="Select the card you wish to discard" imageWidth={80} value={ixianCard} values={(o?.ixianCards ?? []).map(cardChoice)}
                           onChange={(c) => d.set({ _ixianCardId: c ?? -1 })} />
        </>
      );
      break;
    case "Richese": body = <div>Buy a treachery card for <SpiceComponent amount="3" />?</div>; break;
    case "Guild":
      body = (
        <>
          <div>Send up to 4 {forceToken} to any location?</div>
          {guildTargets.length > 0 ? <div className="small">Use the map to choose a location.</div> : <div>There are no valid locations to ship to.</div>}
        </>
      );
      break;
    case "Tleilaxu":
      body = (
        <>
          <div>Revive one of your leaders or up to 4 {forceToken} for free?</div>
          {(o?.tleilaxuMaxAmount ?? 0) > 0 && (
            <div className="d-inline-flex align-items-center">
              <NumberPicker value={tleilaxuForces} min={0} max={o!.tleilaxuMaxAmount} symbolSrc={skin.url(skin.get("FactionForceImage_URL", faction))}
                            onChange={(v) => d.set({ TleilaxuAmountOfForces: v })} />
            </div>
          )}
          <SelectFromImage required={false} imageWidth={80} value={tleilaxuHero}
                           onChange={(h) => d.set({ _tleilaxuHeroId: h ?? -1 })}
                           values={(o?.tleilaxuHeroes ?? []).map((h) => ({ value: h.id, src: h.hero.t === "leader" ? skin.personImage(h.id) : h.hero.t === "treacheryCard" ? skin.cardImage(h.id) : skin.url(skin.str("Messiah_URL")), alt: "", popover: null }))} />
          {tleilaxuHero != null && o?.mayAssignSkill && (
            <Radio label="Do you wish to assign a skill to this leader?" values={[false, true]} labels={["No", "Yes"]} value={assignSkill}
                   onChange={(v) => d.set({ TleilaxuAssignSkill: v })} />
          )}
        </>
      );
      break;
  }
  const ambassadorImage = (a: Ambassador) => <Image src={skin.url(skin.get("AmbassadorImage_URL", a))} alt={skin.get("AmbassadorName_STR", a)} width={30} popover={ambassadorPopup(skin, a)} />;
  const shown = (o?.ambassador ?? "None") as Ambassador;
  return (
    <EventLayout type="AmbassadorActivated" urgent header={<>Activate the {ambassadorImage(shown)} Ambassador?</>}>
      {shown === "BeneGesserit" && (
        <>
          <SelectFromImage label="Activate the effect of which ambassador?" value={actual} onChange={chooseActual}
                           values={(o?.beneGesseritAmbassadors ?? []).map((a) => ({ value: a as Ambassador, src: skin.url(skin.get("AmbassadorImage_URL", a)), alt: skin.get("AmbassadorName_STR", a), popover: ambassadorPopup(skin, a as Ambassador) }))} />
          <br />
        </>
      )}
      {body}
      {o?.mayPass
        ? <ButtonRow confirm={d.confirm} confirmError={d.validation} pass={() => void d.send({ Passed: true })} />
        : <ButtonRow confirm={d.confirm} confirmError={d.validation} />}
    </EventLayout>
  );
}
