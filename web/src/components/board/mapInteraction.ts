// Map interaction: the C# client's MapInteraction.cs, MapPicker.cs and Client.LocationClick.
//
// One action form at a time owns the map. It says which sectors may be clicked as a source, which
// are valid destinations, and what the panel pinned at the chosen destination shows; the map
// layers (MapSelectionHighlights, MapSelectionOverlay, StagedForces) only read this and redraw
// when it changes. Clicks on the map go to every listener, as Client.OnLocationSelected and its
// shift, ctrl and right-click variants do.
//
// For action forms: `useMapPicker` is MapPicker (and MapPickComponent's wiring); forms that draw
// on the map in another way (staged placements) use `mapInteraction` and `onLocationClick`
// directly, as StagedPlacementComponent does.
import { useCallback, useEffect, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import type { Faction } from "../../generated/enums";
import type { Point } from "./geometry";

export interface StagedForces {
  faction: Faction;
  forces: number;
  specialForces: number;
}

export class MapInteraction {
  private listeners = new Set<() => void>();
  private version = 0;

  owner: object | null = null;
  faction: Faction = "None";
  // Locations the owner accepts as a source.
  readonly selectable = new Set<number>();
  // Sources the player has clicked, in click order.
  readonly selected: number[] = [];
  // Destinations that are valid for the selected sources.
  readonly targets = new Set<number>();
  // The destination the player clicked, if any.
  target: number | null = null;
  // Where arrows start when nothing on the map is selected.
  origin: Point | null = null;
  // Where arrows end when no destination is pinned.
  endPoint: Point | null = null;
  // Content shown next to the pinned arrow.
  pin: ReactNode | null = null;
  showCursorArrow = true;
  // Forces placed on the map but not yet confirmed, drawn as ghost tokens.
  readonly staged = new Map<number, StagedForces>();
  onCancel: (() => void) | null = null;

  get canCancel(): boolean {
    return this.owner != null && this.onCancel != null && (this.selected.length > 0 || this.target != null);
  }

  cancel(): void {
    if (this.canCancel) this.onCancel!();
  }

  get isFree(): boolean { return this.owner == null; }
  isOwnedBy(candidate: object): boolean { return this.owner === candidate; }

  claim(owner: object): void {
    this.owner = owner;
    this.reset();
    this.refresh();
  }

  release(owner: object): void {
    if (!this.isOwnedBy(owner)) return;
    this.owner = null;
    this.reset();
    this.refresh();
  }

  // Owners push their whole state after every render of their form, and forms redraw on changes,
  // so only a real change notifies: otherwise a push would redraw the form, which pushes again.
  // The pinned panel is content the owner rebuilds on each render; it has its own listeners
  // (usePin), which are told on every push.
  refresh(): void {
    const signature = JSON.stringify([
      this.owner == null ? 0 : this.ownerId(this.owner), this.faction, [...this.selectable], this.selected, [...this.targets], this.target,
      this.origin, this.endPoint, this.pin != null, this.showCursorArrow, [...this.staged], this.onCancel != null,
    ]);
    for (const l of [...this.pinListeners]) l();
    if (signature === this.signature) return;
    this.signature = signature;
    this.version++;
    for (const l of [...this.listeners]) l();
  }

  private signature = "";
  private owners = new WeakMap<object, number>();
  private nextOwner = 1;
  private ownerId(o: object): number {
    if (!this.owners.has(o)) this.owners.set(o, this.nextOwner++);
    return this.owners.get(o)!;
  }

  private pinListeners = new Set<() => void>();
  private pinVersion = 0;
  subscribePin = (listener: () => void): (() => void) => {
    const l = () => { this.pinVersion++; listener(); };
    this.pinListeners.add(l);
    return () => this.pinListeners.delete(l);
  };
  getPinVersion = (): number => this.pinVersion;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getVersion = (): number => this.version;

  private reset(): void {
    this.selectable.clear();
    this.selected.length = 0;
    this.targets.clear();
    this.target = null;
    this.origin = null;
    this.endPoint = null;
    this.pin = null;
    this.showCursorArrow = true;
    this.staged.clear();
    this.onCancel = null;
  }
}

// Client.MapInteraction: one per page.
export const mapInteraction = new MapInteraction();

// The pinned panel's content, redrawn on every push of the owner.
export function usePin(): ReactNode | null {
  useSyncExternalStore(mapInteraction.subscribePin, mapInteraction.getPinVersion);
  return mapInteraction.pin;
}

// Redraws the caller whenever the map interaction changes; returns it.
export function useMapInteraction(): MapInteraction {
  useSyncExternalStore(mapInteraction.subscribe, mapInteraction.getVersion);
  return mapInteraction;
}

// LocationEventArgs.
export interface LocationClick {
  location: number;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
  right: boolean;
}

const clickListeners = new Set<(c: LocationClick) => void>();

// Every click on a location of the map. Client.LocationClick splits these into OnLocationSelected
// (a plain click), ...WithShift, ...WithCtrlOrAlt and OnLocationRightClicked.
export function onLocationClick(listener: (c: LocationClick) => void): () => void {
  clickListeners.add(listener);
  return () => clickListeners.delete(listener);
}

export function locationClick(c: LocationClick): void {
  for (const l of [...clickListeners]) l(c);
}

// A plain click (Client.OnLocationSelected).
export function isPlainClick(c: LocationClick): boolean {
  return !c.right && !c.shift && !c.ctrl && !c.alt;
}

// The board's click handler for a location.
export function useMapClick() {
  return useCallback((location: number, e: { shiftKey: boolean; ctrlKey: boolean; altKey: boolean }, right = false) => {
    locationClick(right
      ? { location, shift: false, ctrl: false, alt: false, right: true }
      : { location, shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, right: false });
  }, []);
}

export interface MapPickerOptions {
  // Sectors that may be clicked as a source. Leave empty for a plain pick.
  sources?: number[];
  // Locations that may be clicked as the destination, given the current sources.
  targets: (selected: number[]) => number[];
  // Whether the map is needed right now. False hands it back to whoever wants it.
  enabled?: boolean;
  // When the choice is a territory, clicking any sector of the pinned territory unpins it.
  byTerritory?: boolean;
  // territoryOf(location), for byTerritory and for "sources are sectors of one territory".
  territoryOf: (location: number) => number;
  // Content shown next to the pinned destination.
  pin?: (target: number) => ReactNode;
  faction: Faction;
}

export interface MapPick {
  selected: number[];
  target: number | null;
  ownsMap: boolean;
  clear(): void;
}

// MapPicker: drives the map for a form that picks one location, optionally after picking some
// source sectors. Claims the map after render while it is needed and free, releases it when the
// form no longer needs it or goes away.
export function useMapPicker(options: MapPickerOptions): MapPick {
  const self = useRef<object>({});
  const opts = useRef(options);
  opts.current = options;
  const [selected, setSelected] = useState<number[]>([]);
  const [target, setTarget] = useState<number | null>(null);
  const state = useRef({ selected, target });
  state.current = { selected, target };
  const map = mapInteraction;
  // Redraws the form when the map changes hands, not on every change of the map: the form's own
  // renders push to the map, which would otherwise loop.
  const [, setOwned] = useState(false);
  const owns = map.isOwnedBy(self.current);

  const push = useCallback((sel: number[], tgt: number | null) => {
    const o = opts.current;
    if (!map.isOwnedBy(self.current)) return;
    const sources = new Set(o.sources ?? []);
    const keptSel = sel.filter((l) => sources.has(l));
    const targets = new Set(o.targets(keptSel));
    const keptTarget = tgt != null && targets.has(tgt) ? tgt : null;
    map.faction = o.faction;
    map.selectable.clear();
    sources.forEach((l) => map.selectable.add(l));
    map.selected.length = 0;
    map.selected.push(...keptSel);
    map.targets.clear();
    targets.forEach((l) => map.targets.add(l));
    map.target = keptTarget;
    map.showCursorArrow = keptSel.length > 0;
    map.pin = keptTarget != null ? o.pin?.(keptTarget) ?? null : null;
    map.onCancel = () => { setSelected([]); setTarget(null); };
    map.refresh();
  }, [map]);

  // Sync, after every render.
  useEffect(() => {
    const enabled = opts.current.enabled ?? true;
    if (enabled) {
      if (map.isOwnedBy(self.current)) push(state.current.selected, state.current.target);
      else if (map.isFree) { map.claim(self.current); push(state.current.selected, state.current.target); }
    } else if (map.isOwnedBy(self.current)) {
      setSelected([]);
      setTarget(null);
      map.release(self.current);
    }
  });

  // Another form took the map: a half-made selection would be stale. A freed map is claimed.
  useEffect(() => map.subscribe(() => {
    setOwned(map.isOwnedBy(self.current));
    if (map.isOwnedBy(self.current)) return;
    if (map.isFree && (opts.current.enabled ?? true)) {
      map.claim(self.current);
      push(state.current.selected, state.current.target);
    } else if (state.current.selected.length > 0 || state.current.target != null) {
      setSelected([]);
      setTarget(null);
    }
  }), [map, push]);

  useEffect(() => () => map.release(self.current), [map]);

  useEffect(() => onLocationClick((c) => {
    if (!isPlainClick(c) || !map.isOwnedBy(self.current)) return;
    const o = opts.current;
    const l = c.location;
    let sel = state.current.selected;
    let tgt = state.current.target;
    if (sel.includes(l)) {
      sel = sel.filter((s) => s !== l);
      tgt = null;
    } else if ((o.sources ?? []).includes(l)) {
      // Sources are sectors of one territory; a click elsewhere starts over.
      sel = sel.length > 0 && o.territoryOf(sel[0]) !== o.territoryOf(l) ? [l] : [...sel, l];
      tgt = null;
    } else if (tgt != null && (tgt === l || (o.byTerritory && o.territoryOf(tgt) === o.territoryOf(l)))) {
      tgt = null;
    } else if (o.targets(sel).includes(l)) {
      tgt = l;
    } else {
      return;
    }
    setSelected(sel);
    setTarget(tgt);
  }), [map]);

  const clear = useCallback(() => { setSelected([]); setTarget(null); }, []);
  return { selected: owns ? selected : [], target: owns ? target : null, ownsMap: owns, clear };
}
