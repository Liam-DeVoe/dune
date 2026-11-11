// GameEventComponents/PlacementComponent.razor and its subclasses MoveComponent,
// CaravanComponent and FremenRideComponent. A move is made on the map: click the sector to move
// from, then the sector to move to. The force counter and the confirm button appear on the map,
// pinned to the arrow; this panel only explains the next step and offers Pass.
import { useEffect, useRef, useState } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { Faction } from "../../generated/enums";
import type { PlacementOptions } from "../../generated/PlacementOptions";
import { useGame } from "../../state/context";
import { useViewChanges } from "../formDraft";
import { ButtonRow } from "../ButtonRow";
import { PlaceChip } from "../PlaceChip";
import { RichText } from "../RichText";
import { isPlainClick, mapInteraction, onLocationClick, useMapInteraction } from "../board/mapInteraction";
import { EventLayout, forceLocationsString, idOf, MapForceStepper, parseForceLocations, PinChoice, useBoardGeometry, useDraft, useTerritoryOf, Validation, type Draft, type ForceMap } from "./common";

type Kind = "Move" | "Caravan" | "FremenRidesMonster";

interface State {
  forces: ForceMap;
  to: number | null;
  // A sector clicked while it could either join the sources or be the destination.
  undecided: number | null;
  asAdvisors: boolean;
  fromReserves: number;
  specialFromReserves: number;
}

const reset = (): State => ({ forces: [], to: null, undecided: null, asAdvisors: false, fromReserves: 0, specialFromReserves: 0 });

function toDraft(kind: Kind, s: State): Draft {
  const common = { _toId: s.to ?? -1, _forceLocations: forceLocationsString(s.forces), Passed: false };
  return kind === "FremenRidesMonster"
    ? { ForcesFromReserves: s.fromReserves, SpecialForcesFromReserves: s.specialFromReserves, ...common }
    : { AsAdvisors: s.asAdvisors, ...common };
}

// The component's fields, read from a draft (the server's adjusted one once it answered: the
// Bene Gesserit advisor flag follows the destination, as PinContent set it).
function fromDraft(d: Draft, undecided: number | null): State {
  const num = (k: string) => (typeof d[k] === "number" ? (d[k] as number) : 0);
  return {
    forces: parseForceLocations(d._forceLocations),
    to: idOf(d._toId),
    undecided,
    asAdvisors: d.AsAdvisors === true,
    fromReserves: num("ForcesFromReserves"),
    specialFromReserves: num("SpecialForcesFromReserves"),
  };
}

export function PlacementForm({ action, kind }: { action: ActionView; kind: Kind }) {
  const { view, skin } = useGame();
  const faction = (view.viewer.faction ?? "None") as Faction;
  const seat = view.players.find((p) => p.faction === faction)?.seat ?? 0;
  const board = useBoardGeometry();
  const territoryOf = useTerritoryOf();
  const [undecided, setUndecided] = useState<number | null>(null);
  const [, redraw] = useState(0);
  const undecidedRef = useRef(undecided);
  const { draft, options, validation, set, send, confirm, get } = useDraft(action, () => toDraft(kind, reset()));
  const state = fromDraft(draft, undecided);
  const o: PlacementOptions | null = options != null && options.form === kind ? (options as PlacementOptions) : null;
  const self = useRef<object>({});
  const st = { get current() { return fromDraft(get(), undecidedRef.current); } };
  useMapInteraction();
  // The regular move takes the map as soon as it appears. An optional extra move (Caravan)
  // waits until the player asks for it.
  const claimsMapByDefault = kind !== "Caravan";
  const fromReserves = o?.fromReserves ?? false;
  const owns = mapInteraction.isOwnedBy(self.current);

  // PlacementComponent.OnParametersSet runs ResetSelection only when the game object is replaced (an undo
  // or a load): a new event keeps the selection, and useDraft asks the server about it again.
  useViewChanges((change) => { if (change !== "event") update(reset()); });

  const update = (next: State) => {
    undecidedRef.current = next.undecided;
    setUndecided(next.undecided);
    set(toDraft(kind, next));
  };

  const sources = (o?.sources ?? []).map((s) => s.location);
  const sourceForces = (l: number) => o?.sources.find((s) => s.location === l) ?? { location: l, forces: 0, specialForces: 0 };
  const targets = o?.targets ?? [];
  const selected = state.forces.map(([l]) => l);
  const mayAddSource = (l: number) => {
    const s = st.current;
    const territories = new Set(s.forces.map(([x]) => territoryOf(x)));
    return s.forces.length === 0 || territories.has(territoryOf(l)) || ((o?.mayMoveFromTwoTerritories ?? false) && territories.size < 2);
  };
  const addSource = (s: State, l: number): State => {
    const f = sourceForces(l);
    return { ...s, to: null, forces: [...s.forces.filter(([x]) => x !== l), [l, { forces: f.forces, specialForces: f.specialForces }]] };
  };
  // Like a shipment, a ride from reserves opens with one force selected.
  const setDestination = (s: State, l: number): State => {
    if (fromReserves && s.to == null) {
      const forces = Math.min(1, o?.maxFromReserves ?? 0);
      const special = forces === 0 ? Math.min(1, o?.maxSpecialFromReserves ?? 0) : 0;
      return { ...s, to: l, fromReserves: forces, specialFromReserves: special };
    }
    return { ...s, to: l };
  };

  useEffect(() => onLocationClick((c) => {
    if (!isPlainClick(c) || !mapInteraction.isOwnedBy(self.current)) return;
    const s = { ...st.current, undecided: null };
    const l = c.location;
    let next: State | null = null;
    if (s.forces.some(([x]) => x === l)) {
      next = { ...s, forces: s.forces.filter(([x]) => x !== l), to: null };
    } else if (s.to === l) {
      next = { ...s, to: null };
    } else if (targets.includes(l) && sources.includes(l) && s.forces.some(([x]) => territoryOf(x) === territoryOf(l))) {
      // Another sector of a source territory could join the sources or be the destination.
      next = { ...s, undecided: l };
    } else if (targets.includes(l)) {
      next = setDestination(s, l);
    } else if (sources.includes(l)) {
      next = addSource(mayAddSource(l) ? s : { ...s, forces: [] }, l);
    }
    if (next) update(next);
  }));

  const clearSelection = () => update(reset());
  const decide = (asDestination: boolean) => {
    const s = st.current;
    const l = s.undecided;
    if (l == null) return;
    const cleared = { ...s, undecided: null };
    update(asDestination ? setDestination(cleared, l) : addSource(cleared, l));
  };
  const setAmount = (l: number, amount: number, special: boolean) => {
    update({ ...st.current, forces: st.current.forces.map(([x, b]) => [x, x === l ? (special ? { ...b, specialForces: amount } : { ...b, forces: amount }) : b]) });
  };

  const choicePin = state.undecided != null && (
    <div className="pin-body">
      <div className="pin-from"><PlaceChip place={{ location: state.undecided }} /></div>
      <div className="pin-actions">
        <button type="button" className="btn btn-secondary" onClick={() => decide(false)}>Add as source</button>
        <button type="button" className="btn btn-primary" onClick={() => decide(true)}>Move here</button>
      </div>
    </div>
  );

  const title = kind === "Move" ? "Move" : kind === "Caravan" ? `Perform a ${skin.enumValue("TreacheryCardType", "Caravan")}` : "";
  const confirmLabel = kind === "FremenRidesMonster" ? "Ride!" : title;

  const pin = () => {
    const s = state;
    return (
      <div className="pin-body">
        {s.forces.map(([l, b]) => {
          const f = sourceForces(l);
          return (
            <div key={l} className="pin-row">
              {s.forces.length > 1 && <span className="pin-from"><PlaceChip place={{ location: l }} /></span>}
              {f.forces > 0 && <MapForceStepper value={b.forces} onChange={(v) => setAmount(l, v, false)} max={f.forces} faction={faction} />}
              {f.specialForces > 0 && <MapForceStepper value={b.specialForces} onChange={(v) => setAmount(l, v, true)} max={f.specialForces} faction={faction} special />}
            </div>
          );
        })}
        {fromReserves && (
          <div className="pin-row">
            {(o?.maxFromReserves ?? 0) > 0 && (
              <MapForceStepper value={s.fromReserves} onChange={(v) => update({ ...st.current, fromReserves: v })} max={o!.maxFromReserves} faction={faction} />
            )}
            {(o?.maxSpecialFromReserves ?? 0) > 0 && (
              <MapForceStepper value={s.specialFromReserves} onChange={(v) => update({ ...st.current, specialFromReserves: v })} max={o!.maxSpecialFromReserves} faction={faction} special />
            )}
          </div>
        )}
        {o?.advisorChoice && (
          <PinChoice values={[false, true]} value={s.asAdvisors} onChange={(v) => update({ ...st.current, asAdvisors: v })} label={(v) => (v ? "Advisors" : "Fighters")} />
        )}
        {o?.conflictingAlly && <div className="pin-warn">Your ally is there: all your forces there at the end of your move will be destroyed.</div>}
        <Validation parts={validation} />
        <div className="pin-actions">
          <button type="button" className="btn btn-secondary" onClick={clearSelection}>Cancel</button>
          <button type="button" className="btn btn-primary" disabled={validation != null} onClick={confirm}>{confirmLabel}</button>
        </div>
      </div>
    );
  };

  // PushToMap after every render while owning the map; ClaimIfFree for the regular move.
  useEffect(() => {
    const map = mapInteraction;
    if (!map.isOwnedBy(self.current)) {
      if (!claimsMapByDefault || !map.isFree) return;
      map.claim(self.current);
    }
    const s = state;
    map.faction = faction;
    map.selectable.clear();
    sources.filter((l) => board.hasCenter(l)).forEach((l) => map.selectable.add(l));
    map.selected.length = 0;
    map.selected.push(...selected);
    map.targets.clear();
    targets.forEach((l) => map.targets.add(l));
    map.target = s.to;
    map.origin = fromReserves ? board.playerTokenPosition(seat) : null;
    map.endPoint = s.undecided != null && board.hasCenter(s.undecided) ? board.getCenter(s.undecided) : null;
    map.pin = s.to != null ? pin() : s.undecided != null ? choicePin : null;
    map.onCancel = clearSelection;
    map.refresh();
  });
  // HandleMapChanged: claim a freed map, or drop a stale selection when another form took it.
  useEffect(() => mapInteraction.subscribe(() => {
    if (mapInteraction.isFree && claimsMapByDefault) {
      mapInteraction.claim(self.current);
    } else if (!mapInteraction.isOwnedBy(self.current) && (st.current.forces.length > 0 || st.current.to != null)) {
      update(reset());
    }
  }), [claimsMapByDefault]);
  useEffect(() => () => mapInteraction.release(self.current), []);

  const claimMap = () => {
    mapInteraction.claim(self.current);
    redraw((n) => n + 1);
  };
  const releaseMap = () => {
    update(reset());
    mapInteraction.release(self.current);
  };

  const mayPass = kind !== "Caravan";
  const pass = mayPass ? () => void send({ Passed: true }) : undefined;
  const header = kind === "FremenRidesMonster" ? (o?.title ? <RichText parts={o.title} size={20} /> : null) : <>{title}{"?"}</>;
  return (
    <EventLayout type={kind} urgent header={header}>
      {o?.informAboutCaravan && (
        <div className="text-info small">Note: if you wish to perform a {skin.enumValue("TreacheryCardType", "Caravan")} this turn, you need to do that before performing your regular move.</div>
      )}
      {kind === "FremenRidesMonster" ? (
        <>
          <div className="small">Use the map to ride the worm.</div>
          <ButtonRow pass={pass} passText="Pass" />
        </>
      ) : sources.length > 0 ? (
        <>
          {owns ? (
            <>
              <div className="small">Use the map to move forces.</div>
              {!claimsMapByDefault && <button className="btn btn-secondary btn-sm mt-1" onClick={releaseMap}>Cancel</button>}
            </>
          ) : claimsMapByDefault ? (
            <div className="small text-2">Finish the extra move on the map first, or cancel it.</div>
          ) : (
            <button className="btn btn-secondary btn-sm" onClick={claimMap}>{`Play ${skin.enumValue("TreacheryCardType", "Caravan")}`}</button>
          )}
          {mayPass && <ButtonRow pass={pass} passText="Pass" />}
        </>
      ) : (
        mayPass && <ButtonRow pass={pass} passText="Pass" />
      )}
    </EventLayout>
  );
}
