// GameEventComponents/ShipmentComponent.razor. A shipment is made on the map. From reserves:
// click the destination, and the arrow runs from your token on the map's edge. Site-to-site
// (Guild): click the sector to ship from, then the destination. Back to reserves: click the
// sector, then "To reserves" in the pinned panel. Counters, cost and every option that applies
// live in the panel pinned to the arrow; this panel only explains the situation and offers Pass.
import { useEffect, useRef } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { ShipmentOptions } from "../../generated/ShipmentOptions";
import { type Faction, type ShipmentType, ShipmentTypeValue } from "../../generated/enums";
import { useGame } from "../../state/context";
import { useViewChanges } from "../formDraft";
import { ButtonRow } from "../ButtonRow";
import { FactionChip } from "../FactionChip";
import { PlaceChip } from "../PlaceChip";
import { isPlainClick, mapInteraction, onLocationClick, useMapInteraction } from "../board/mapInteraction";
import { enumName, EventLayout, forceLocationsString, idOf, MapForceStepper, optionsOf, parseForceLocations, PinChoice, useBoardGeometry, useDraft, Validation, type Draft, type ForceMap } from "./common";

interface State {
  type: ShipmentType;
  karmaCard: number | null;
  forces: number;
  specialForces: number;
  ally: number;
  noField: number;
  cunningNoField: number;
  smuggled: number;
  smuggledSpecial: number;
  origins: ForceMap;
  from: number | null;
  to: number | null;
}

// ResetSelection.
const reset = (): State => ({
  type: "ShipmentNormal", karmaCard: null, forces: 0, specialForces: 0, ally: 0, noField: -1, cunningNoField: -1,
  smuggled: 0, smuggledSpecial: 0, origins: [], from: null, to: null,
});

// ConfirmedResult, in savegame JSON (NoFieldValue is stored plus one).
function toDraft(s: State): Draft {
  const direction = s.type === "ShipmentBack" ? -1 : 1;
  return {
    ShipmentType: ShipmentTypeValue[s.type],
    ForceAmount: direction * s.forces,
    SpecialForceAmount: direction * s.specialForces,
    SmuggledAmount: s.smuggled,
    SmuggledSpecialAmount: s.smuggledSpecial,
    AllyContributionAmount: s.ally,
    UseRicheseSecretAlly: s.type === "ShipmentRicheseSecretAlly",
    _toId: s.to ?? -1,
    _fromId: s.from ?? -1,
    _noFieldValue: s.noField + 1,
    _cunningNoFieldValue: s.cunningNoField + 1,
    _karmaCardId: s.karmaCard ?? -1,
    _forceLocations: forceLocationsString(s.origins),
    Passed: false,
  };
}

// The source sector of a site-to-site or back-to-reserves shipment: From for the former, To
// for the latter.
const sourceOf = (s: State) => (s.type === "ShipmentSiteToSite" ? s.from : s.type === "ShipmentBack" ? s.to : null);
const destinationOf = (s: State) => (s.type === "ShipmentBack" ? null : s.to);

// The component's fields, read from a draft (the server's adjusted one once it answered).
function fromDraft(d: Draft): State {
  const num = (k: string) => (typeof d[k] === "number" ? (d[k] as number) : 0);
  return {
    type: enumName(ShipmentTypeValue, d.ShipmentType, "ShipmentNormal"),
    karmaCard: idOf(d._karmaCardId),
    forces: Math.abs(num("ForceAmount")),
    specialForces: Math.abs(num("SpecialForceAmount")),
    ally: num("AllyContributionAmount"),
    noField: num("_noFieldValue") - 1,
    cunningNoField: num("_cunningNoFieldValue") - 1,
    smuggled: num("SmuggledAmount"),
    smuggledSpecial: num("SmuggledSpecialAmount"),
    origins: parseForceLocations(d._forceLocations),
    from: idOf(d._fromId),
    to: idOf(d._toId),
  };
}

export function ShipmentForm({ action }: { action: ActionView }) {
  const { view, skin } = useGame();
  const faction = (view.viewer.faction ?? "None") as Faction;
  const seat = view.players.find((p) => p.faction === faction)?.seat ?? 0;
  const me = view.players.find((p) => p.faction === faction);
  const board = useBoardGeometry();
  const { draft, options, validation, set, send, confirm, get } = useDraft(action, () => toDraft(reset()));
  const state = fromDraft(draft);
  const o = optionsOf(options, "Shipment");
  const self = useRef<object>({});
  const st = { get current() { return fromDraft(get()); } };
  const oRef = useRef(o);
  oRef.current = o;
  useMapInteraction();

  // ShipmentComponent.OnParametersSet runs ResetSelection only when the game object is replaced (an undo
  // or a load): a new event keeps the selection, and useDraft asks the server about it again.
  useViewChanges((change) => { if (change !== "event") update(reset()); });

  // The server answers with the draft as PinContent leaves it (an input that no longer applies
  // dropped), so the form only sends what was clicked.
  const update = (next: State, then?: (s: State, o: ShipmentOptions) => State | null) => {
    set(toDraft(next), then && ((answer, d) => {
      const so = optionsOf(answer, "Shipment");
      const r = so ? then(fromDraft(d), so) : null;
      return r ? toDraft(r) : null;
    }));
  };

  const onMap = (l: number) => board.hasCenter(l);
  const prevented = o?.preventedFromShipping ?? false;
  const maySelectSource = o != null && (o.mayShipCrossPlanet || o.mayShipToReserves);
  const selectableSources = maySelectSource ? o.sources.map((s) => s.location).filter(onMap) : [];
  const validLocations = (o?.targets ?? []).filter(onMap);

  // InitializeAmounts: one force of whichever kind is available, so the panel opens ready to
  // confirm. ValidShipmentForces depends on the new type, source and destination, so this runs
  // on the options of the new draft, once they arrive.
  const initializeAmounts = (s: State, so: ShipmentOptions): State => {
    const forces = Math.min(1, so.maxForces);
    let specialForces = forces === 0 ? Math.min(1, so.maxSpecialForces) : 0;
    if (faction === "Richese" && s.noField >= 0) specialForces = 1;
    return { ...s, forces, specialForces };
  };
  const initialized = (next: State) => update(next, initializeAmounts);

  const clearSelection = () => update(reset());

  // HandleLocationSelected.
  useEffect(() => onLocationClick((c) => {
    if (!isPlainClick(c) || !mapInteraction.isOwnedBy(self.current) || prevented) return;
    const s = st.current;
    const l = c.location;
    const source = sourceOf(s);
    const pickDestination = () => (s.to == null ? initialized({ ...s, to: l }) : update({ ...s, to: l }));
    if (source === l) {
      update(reset());
    } else if (destinationOf(s) === l) {
      update({ ...s, to: null });
    } else if (source != null && s.type !== "ShipmentBack" && validLocations.includes(l)) {
      pickDestination();
    } else if (selectableSources.includes(l)) {
      if (oRef.current?.mayShipCrossPlanet) update({ ...reset(), type: "ShipmentSiteToSite", from: l });
      else initialized({ ...reset(), type: "ShipmentBack", to: l });
    } else if (s.type !== "ShipmentBack" && validLocations.includes(l)) {
      pickDestination();
    }
  }));

  const shipHereFromReserves = () => initialized({ ...reset(), to: st.current.from });
  const shipBackToReserves = () => initialized({ ...reset(), type: "ShipmentBack", to: st.current.from });
  // SetShipmentType: the destination stays if the new type may ship there, which the options of
  // the new draft say.
  const setType = (type: ShipmentType) => {
    const s = st.current;
    if (s.type === type) return;
    update({ ...reset(), type, to: s.to }, (n, so) => {
      if (n.to == null) return null;
      return so.targets.includes(n.to) && board.hasCenter(n.to) ? initializeAmounts(n, so) : { ...n, to: null };
    });
  };

  const setNoField = (value: number, cunning: boolean) => {
    const s = { ...st.current };
    if (cunning) {
      s.cunningNoField = value;
    } else {
      s.noField = value;
      if (value < 0) s.cunningNoField = -1;
      if (faction === "Richese") {
        // DefaultNoFieldSpecialForceAmount(Faction.Richese, value): 1 with a No-Field, else 0.
        s.specialForces = value >= 0 ? 1 : 0;
        if (value >= 0 && !(o?.richeseHighThreshold ?? false)) s.forces = 0;
      }
    }
    // ValidShipmentForces for the new No-Field: clamp once its options arrive.
    update(s, (n, so) => {
      const clamped = { ...n, forces: Math.min(n.forces, so.maxForces), specialForces: Math.min(n.specialForces, so.maxSpecialForces) };
      return clamped.forces !== n.forces || clamped.specialForces !== n.specialForces ? clamped : null;
    });
  };
  const setOrigin = (world: number, amount: number, special: boolean) => {
    const origins: ForceMap = st.current.origins.map(([l, b]) => [l, { ...b }]);
    let entry = origins.find(([l]) => l === world);
    if (!entry) { entry = [world, { forces: 0, specialForces: 0 }]; origins.push(entry); }
    if (special) entry[1].specialForces = amount; else entry[1].forces = amount;
    update({ ...st.current, origins });
  };
  const change = (c: Partial<State>) => update({ ...st.current, ...c });

  const sourcePin = (
    <div className="pin-body">
      <div className="pin-hint">Click where to ship to</div>
      <div className="pin-actions">
        <button type="button" className="btn btn-secondary" onClick={clearSelection}>Cancel</button>
        {o?.mayShipHereFromReserves && <button type="button" className="btn btn-secondary" onClick={shipHereFromReserves}>Here from reserves</button>}
        {o?.mayShipToReserves && <button type="button" className="btn btn-secondary" onClick={shipBackToReserves}>To reserves</button>}
      </div>
    </div>
  );

  const pin = () => {
    if (!o) return null;
    const s = state;
    const canChooseType = (s.type === "ShipmentNormal" || s.type === "ShipmentRicheseSecretAlly") && o.mayUseRicheseSecretAlly;
    const richeseNoField = faction === "Richese" && s.noField >= 0 && !o.richeseHighThreshold;
    const fromReserves = s.type === "ShipmentNormal" || s.type === "ShipmentRicheseSecretAlly";
    const worlds = fromReserves ? o.homeworlds : [];
    const chooseNormal = worlds.filter((w) => w.normal).length > 1;
    const chooseSpecial = worlds.filter((w) => w.special).length > 1;
    return (
      <div className="pin-body">
        {s.type === "ShipmentBack" && s.to != null && <div className="pin-hint">Back to reserves from <PlaceChip place={{ location: s.to }} /></div>}
        {canChooseType && (
          <PinChoice values={["ShipmentNormal", "ShipmentRicheseSecretAlly"] as ShipmentType[]} value={s.type} onChange={setType}
                     label={(t) => (t === "ShipmentNormal" ? "Normal" : `${skin.faction("Richese")} secret ally`)} />
        )}
        {o.mayUseNoField && fromReserves && (
          <>
            <PinChoice values={[-1, ...o.noFieldValues]} value={s.noField} onChange={(v) => setNoField(v, false)}
                       label={(v) => (v < 0 ? "No No-Field" : `No-Field ${v}`)} />
            {s.noField >= 0 && o.mayUseCunningNoField && (
              <PinChoice values={[-1, ...o.cunningNoFieldValues]} value={s.cunningNoField} onChange={(v) => setNoField(v, true)}
                         label={(v) => (v < 0 ? "No second No-Field" : `Cunning No-Field ${v}`)} />
            )}
          </>
        )}
        <div className="pin-row">
          {o.maxForces > 0 && !richeseNoField && (
            <MapForceStepper value={s.forces} onChange={(v) => change({ forces: v })} max={o.maxForces} faction={faction} />
          )}
          {o.maxSpecialForces > 0 && faction !== "Richese" && (
            <MapForceStepper value={s.specialForces} onChange={(v) => change({ specialForces: v })} max={o.maxSpecialForces} faction={faction} special />
          )}
        </div>
        {(chooseNormal || chooseSpecial) && (
          <>
            <div className="pin-hint">From which worlds:</div>
            {worlds.map((w) => {
              const origin = s.origins.find(([l]) => l === w.location)?.[1];
              return (
                <div key={w.location} className="pin-row">
                  <span className="pin-from">{w.label}</span>
                  {chooseNormal && w.forces > 0 && (
                    <MapForceStepper value={origin?.forces ?? 0} onChange={(v) => setOrigin(w.location, v, false)} max={w.forces} faction={faction} />
                  )}
                  {chooseSpecial && w.specialForces > 0 && (
                    <MapForceStepper value={origin?.specialForces ?? 0} onChange={(v) => setOrigin(w.location, v, true)} max={w.specialForces} faction={faction} special />
                  )}
                </div>
              );
            })}
          </>
        )}
        {fromReserves && faction !== "Fremen" && (s.forces > 0 || s.specialForces > 0) && o.maySmuggle && (
          <div className="pin-row">
            <span className="pin-from">Smuggle 1 for free</span>
            {(me?.forcesInReserve ?? 0) > 0 && <MapForceStepper value={s.smuggled} onChange={(v) => change({ smuggled: v })} max={1} faction={faction} />}
            {(me?.specialForcesInReserve ?? 0) > 0 && faction !== "Richese" && (
              <MapForceStepper value={s.smuggledSpecial} onChange={(v) => change({ smuggledSpecial: v })} max={1} faction={faction} special />
            )}
          </div>
        )}
        {o.mayKarma && (
          <>
            <div className="pin-hint">{skin.enumValue("TreacheryCardType", "Karma")} for <FactionChip faction="Guild" /> rate:</div>
            <PinChoice values={[null, ...o.karmaCards.map((k) => k.card)]} value={s.karmaCard} onChange={(v) => change({ karmaCard: v })}
                       label={(v) => (v == null ? "None" : `${skin.card(v)}${o.karmaCards.find((k) => k.card === v)?.own ? "" : " (ally)"}`)} />
          </>
        )}
        <div className="pin-row pin-cost">
          <img src={skin.url(skin.str("Resource_URL"))} alt="" />
          <span>Cost {o.cost}</span>
          {o.allyCanPay > 0 && (
            <>
              <span className="pin-from">ally pays</span>
              <MapForceStepper value={s.ally} onChange={(v) => change({ ally: v })} max={Math.min(o.allyCanPay, o.cost)}
                               faction={(me?.ally ?? "None") as Faction} spice />
            </>
          )}
        </div>
        {destinationOf(s) != null && o.conflictingAlly && (
          <div className="pin-warn">Your ally is there: all your forces there at the end of your move will be destroyed.</div>
        )}
        <Validation parts={validation} />
        <div className="pin-actions">
          <button type="button" className="btn btn-secondary" onClick={clearSelection}>Cancel</button>
          <button type="button" className="btn btn-primary" disabled={validation != null} onClick={confirm}>Ship</button>
        </div>
      </div>
    );
  };

  // ClaimIfFree, HandleMapChanged and PushToMap, after every render.
  useEffect(() => {
    const map = mapInteraction;
    if (!map.isOwnedBy(self.current)) {
      if (!map.isFree) return;
      map.claim(self.current);
    }
    const s = state;
    map.faction = faction;
    map.selectable.clear();
    map.selected.length = 0;
    map.targets.clear();
    map.target = null;
    map.origin = null;
    map.endPoint = null;
    map.pin = null;
    map.showCursorArrow = true;
    if (!prevented) {
      selectableSources.forEach((l) => map.selectable.add(l));
      const token = board.playerTokenPosition(seat);
      const source = sourceOf(s);
      if (source != null) map.selected.push(source);
      else map.origin = token;
      if (s.type === "ShipmentBack") {
        map.endPoint = token;
        map.showCursorArrow = false;
      } else {
        validLocations.forEach((l) => map.targets.add(l));
        map.target = destinationOf(s);
      }
      if (destinationOf(s) != null || s.type === "ShipmentBack") map.pin = pin();
      else if (source != null) map.pin = sourcePin;
    }
    map.onCancel = clearSelection;
    map.refresh();
  });
  useEffect(() => mapInteraction.subscribe(() => {
    if (mapInteraction.isFree) {
      mapInteraction.claim(self.current);
    }
  }), []);
  useEffect(() => () => mapInteraction.release(self.current), []);

  const fremen = faction === "Fremen";
  return (
    <EventLayout type="Shipment" urgent header="Ship?">
      {fremen && o && (
        <div className="small text-2">You may bring any number of forces onto <PlaceChip place={{ territory: o.greatFlat }} /> or onto any one territory within two territories of it, for free.</div>
      )}
      {fremen && o?.mayShipCrossPlanet && <div className="small text-2">Your ally also lets you ship elsewhere from your reserves.</div>}
      {o?.mayShipCrossPlanet && <div className="small text-2">You may also ship site-to-site: click the sector to ship from first.</div>}
      {o?.mayShipToReserves && <div className="small text-2">You may also ship forces back to your reserves: click the sector holding them.</div>}
      {prevented ? <div className="small">{o?.karmaName} prevents you from shipping.</div> : <div className="small">Use the map to ship forces.</div>}
      {state.type !== "ShipmentRicheseSecretAlly" && <ButtonRow pass={() => void send({ Passed: true })} passText="Pass" />}
    </EventLayout>
  );
}

