// GameEventComponents/PerformSetupComponent.razor (a StagedPlacementComponent): custom setup. The
// host places each faction's starting forces on the map in turn, and sets its starting spice.
// Click a sector for a force, shift-click for a special force, right-click to take one back. The
// list below the instructions is a fallback for people who cannot right-click.
import { useEffect, useRef, useState } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { Faction } from "../../generated/enums";
import { useGame } from "../../state/context";
import { isPlainClick, mapInteraction, onLocationClick, useMapInteraction } from "../board/mapInteraction";
import { ButtonRow } from "../ButtonRow";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { SelectResources } from "../NumberPicker";
import { PlaceChip } from "../PlaceChip";
import { EventLayout, forceLocationsString, type ForceMap, MapForceStepper, optionsOf, useDraft, useForceNames, Validation } from "../shipmove/common";

export function PerformSetupForm({ action }: { action: ActionView }) {
  const { skin } = useGame();
  const o = optionsOf(action.options, "PerformSetup");
  const faction = (o?.faction ?? "None") as Faction;
  const names = useForceNames(faction);
  const [forces, setForces] = useState<ForceMap>([]);
  const [resources, setResources] = useState(0);
  const history = useRef<Array<[number, boolean]>>([]);
  const st = useRef(forces);
  st.current = forces;
  const self = useRef<object>({});
  const { validation, set, confirm } = useDraft(action, () => ({ Passed: false, Resources: 0, _forceLocations: "" }));
  useMapInteraction();

  // SyncPlacer: when the next faction to set up changes, start again with nothing staged.
  const placer = useRef(faction);
  useEffect(() => {
    if (placer.current === faction) return;
    placer.current = faction;
    history.current = [];
    st.current = [];
    setForces([]);
    setResources(0);
    set({ Passed: false, Resources: 0, _forceLocations: "" });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [faction]);

  const staged = (special: boolean) => st.current.reduce((n, [, b]) => n + (special ? b.specialForces : b.forces), 0);
  // StagedPlacementComponent.Room: reserves left, no fixed total.
  const room = (special: boolean) => Math.max(0, special ? (o?.specialForcesInReserve ?? 0) - staged(true) : (o?.forcesInReserve ?? 0) - staged(false));
  const commit = (next: ForceMap) => {
    st.current = next;
    setForces(next);
    set({ _forceLocations: forceLocationsString(next) });
  };
  const add = (l: number, special: boolean) => {
    if (!(o?.locations ?? []).includes(l) || (special && !o?.mayStageSpecial) || room(special) === 0) return;
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
  // PushToMap after every render while owning the map; ClaimIfFree.
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
    (o?.locations ?? []).forEach((l) => map.targets.add(l));
    map.staged.clear();
    for (const [l, b] of forces) map.staged.set(l, { faction, forces: b.forces, specialForces: b.specialForces });
    map.refresh();
  });
  useEffect(() => mapInteraction.subscribe(() => { if (mapInteraction.isFree) mapInteraction.claim(self.current); }), []);
  useEffect(() => () => mapInteraction.release(self.current), []);

  const shown = [...forces].filter(([, b]) => b.forces + b.specialForces > 0).sort(([a], [b]) => skin.location(a).localeCompare(skin.location(b)));
  const step = (l: number, value: number, special: boolean) => {
    const b = forces.find(([x]) => x === l)?.[1];
    const current = special ? b?.specialForces ?? 0 : b?.forces ?? 0;
    if (value > current) add(l, special);
    else if (value < current) remove(l, special);
  };
  const spice = skin.concept("Resource");
  return (
    <EventLayout type="PerformSetup" urgent header={<>Set <FactionChip faction={faction} /> initial {spice} and forces.</>}>
      <SelectResources value={resources} onChange={(v) => { setResources(v); set({ Resources: v }); }} min={0} max={o?.maxResources ?? 100} label={spice} />
      <div className="small mt-2">Use the map to place forces.</div>
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
