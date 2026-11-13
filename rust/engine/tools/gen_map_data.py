#!/usr/bin/env python3
"""Generates rust/engine/src/map/data.rs from treachery-online/Shared/Map.cs.

The output mirrors Map.InitializeLocations and Map.InitializeLocationNeighbours
as static tables. tests/static_data.rs checks the result against the C# dump.

Usage: python3 tools/gen_map_data.py (from rust/engine)
"""
import re
import sys
from pathlib import Path

ENGINE = Path(__file__).resolve().parent.parent
MAP_CS = ENGINE.parent.parent / "treachery-online" / "Shared" / "Map.cs"
OUT = ENGINE / "src" / "map" / "data.rs"

src = MAP_CS.read_text(encoding="utf-8-sig")
init = src[src.index("private void InitializeLocations()"):src.index("private void AddHomeworld(")]
neigh = src[src.index("private void InitializeLocationNeighbours()"):src.index("private struct NeighbourCacheKey")]


def body_at(text, open_idx):
    depth = 0
    for i in range(open_idx, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[open_idx + 1:i]
    raise ValueError("unbalanced")


def props(body):
    body = re.sub(r"//[^\n]*", "", body)
    out, depth, cur = {}, 0, ""
    for ch in body + ",":
        if ch in "{(":
            depth += 1
        elif ch in "})":
            depth -= 1
        if ch == "," and depth == 0:
            if "=" in cur:
                k, v = cur.split("=", 1)
                out[k.strip()] = v.strip()
            cur = ""
        else:
            cur += ch
    return out


territories = {}   # id -> props
locations = []     # dicts in _locations order
names_t, names_l = {}, {}
var_t = {}
creation_events = []

pat = re.compile(r"(?:(var\s+)?(\w+)\s*=\s*)?new (Territory|Location)\((\d+)\)\s*\{")
for m in pat.finditer(init):
    kind, ident = m.group(3), int(m.group(4))
    body = body_at(init, m.end() - 1)
    p = props(body)
    if kind == "Territory":
        territories[ident] = p
        if m.group(2) and m.group(2) != "Territory":
            if m.group(1):
                var_t[m.group(2)] = ident
            else:
                names_t[m.group(2)] = ident
        creation_events.append(("t", ident, m.start()))
    else:
        if m.group(2):
            names_l[m.group(2)] = ident
        creation_events.append(("l", ident, m.start(), p))

last_t = None
for ev in sorted(creation_events, key=lambda e: e[2]):
    if ev[0] == "t":
        last_t = ev[1]
        continue
    _, ident, pos, p = ev
    tv = p["Territory"]
    mt = re.match(r"new Territory\((\d+)\)", tv)
    if mt:
        tid = int(mt.group(1))
    elif tv in names_t:
        tid = names_t[tv]
    else:
        tid = last_t  # `t` declared in the same block
        assert tv == "t", tv
    locations.append(dict(id=ident, territory=tid, kind="Normal",
                          orientation=p.get("Orientation", '""'),
                          sector=int(p.get("Sector", "0")),
                          spice=int(p.get("SpiceBlowAmount", "0")),
                          dtt=p.get("DiscoveryTokenType", "DiscoveryTokenType.None").split(".")[1]))

# Hidden mobile stronghold
m = re.search(r"HiddenMobileStronghold = new HiddenMobileStronghold\(t, (\d+)\) \{ SpiceBlowAmount = (\d+) \}", init)
hms_tid = max(territories)  # the `var t` of that block
hms_block_t = re.search(r"var t = new Territory\((\d+)\)[^}]*\}[^}]*HiddenMobileStronghold = new", init, re.S)
locations.append(dict(id=int(m.group(1)), territory=int(hms_block_t.group(1)), kind="HiddenMobileStronghold",
                      orientation='""', sector=0, spice=int(m.group(2)), dtt="None"))
names_l["HiddenMobileStronghold"] = int(m.group(1))

for m in re.finditer(r"AddHomeworld\((\d+), (\d+), World\.(\w+), Faction\.(\w+), (\w+), (\w+), (\d+), (\d+), (\d+), (\d+)\);", init):
    tid, lid = int(m.group(1)), int(m.group(2))
    territories[tid] = {"IsHomeworld": "true", "HasReducedShippingCost": "true", "IsStronghold": "false", "IsDiscovery": "false",
                        "IsProtectedFromStorm": "true", "IsProtectedFromWorm": "true"}
    locations.append(dict(id=lid, territory=tid, kind="Homeworld", orientation='""', sector=-1, spice=0, dtt="None",
                          hw=m.groups()[2:]))

for m in re.finditer(r"(\w+) = AddDiscoveredLocation\((\d+), (\d+), DiscoveryToken\.(\w+), (\w+)\);", init):
    tid, lid = int(m.group(2)), int(m.group(3))
    # new Territory(false, isStronghold, true, true, true, true, territoryId):
    # (isHomeworld, isStronghold, hasReducedShippingCost, isDiscovery, isProtectedFromStorm, isProtectedFromWorm, id)
    territories[tid] = {"IsHomeworld": "false", "IsStronghold": m.group(5), "HasReducedShippingCost": "true", "IsDiscovery": "true",
                        "IsProtectedFromStorm": "true", "IsProtectedFromWorm": "true"}
    locations.append(dict(id=lid, territory=tid, kind="Discovered", orientation='""', sector=0, spice=0, dtt="None",
                          discovery=m.group(4)))
    names_l[m.group(1)] = lid

assert [l["id"] for l in locations] == list(range(len(locations))), "location ids must equal their index"

# Territory -> contiguous location range
ranges = {}
for idx, l in enumerate(locations):
    t = l["territory"]
    if t in ranges:
        first, count = ranges[t]
        assert first + count == idx, f"territory {t} locations not contiguous"
        ranges[t] = (first, count + 1)
    else:
        ranges[t] = (idx, 1)

pairs = [(int(a), int(b)) for a, b in re.findall(r"_locations\[(\d+)\]\.Neighbours\.Add\(_locations\[(\d+)\]\);", neigh)]


def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def b(v):
    return {"true": "true", "false": "false"}[v]


out = []
w = out.append
w("//! Static data of `Map.InitializeLocations` and `Map.InitializeLocationNeighbours`.")
w("//! GENERATED by tools/gen_map_data.py from Shared/Map.cs. Do not edit by hand.")
w("//! Checked against the C# runtime by tests/static_data.rs.")
w("")
w("use crate::enumerations::*;")
w("use crate::map::{LocationData, LocationKind, HomeworldData, TerritoryData};")
w("use crate::location::Location;")
w("use crate::territory::Territory;")
w("")
w(f"pub(crate) const LOCATION_COUNT: usize = {len(locations)};")
w(f"pub(crate) const TERRITORY_COUNT: usize = {len(territories)};")
w("")
w("/// Territories in order of id (which is also the order of `Map.Territories(true)`).")
w("pub(crate) static TERRITORIES: [TerritoryData; TERRITORY_COUNT] = [")
for tid in sorted(territories):
    p = territories[tid]
    first, count = ranges[tid]
    adv = p.get("Advantage", "StrongholdAdvantage.None").split(".")[1]
    w(f"    TerritoryData {{ id: {tid}, is_homeworld: {b(p.get('IsHomeworld', 'false'))}, has_reduced_shipping_cost: {b(p.get('HasReducedShippingCost', 'false'))}, "
      f"is_discovery: {b(p.get('IsDiscovery', 'false'))}, is_stronghold: {b(p.get('IsStronghold', 'false'))}, "
      f"is_protected_from_storm: {b(p.get('IsProtectedFromStorm', 'false'))}, is_protected_from_worm: {b(p.get('IsProtectedFromWorm', 'false'))}, "
      f"advantage: StrongholdAdvantage::{adv}, first_location: {first}, location_count: {count} }},")
w("];")
w("")
w("/// `Map._locations`, in insertion order. Index == `Location.Id`.")
w("pub(crate) static LOCATIONS: [LocationData; LOCATION_COUNT] = [")
for l in locations:
    if l["kind"] == "Normal":
        kind = "LocationKind::Normal"
    elif l["kind"] == "HiddenMobileStronghold":
        kind = "LocationKind::HiddenMobileStronghold"
    elif l["kind"] == "Discovered":
        kind = f"LocationKind::Discovered {{ discovery: DiscoveryToken::{l['discovery']} }}"
    else:
        world, faction, normal, special, thr, hi, lo, res = l["hw"]
        kind = (f"LocationKind::Homeworld(HomeworldData {{ world: World::{world}, faction: Faction::{faction}, is_home_of_normal_forces: {normal}, "
                f"is_home_of_special_forces: {special}, threshold: {thr}, battle_bonus_and_lasgun_shield_limit_at_high_threshold: {hi}, "
                f"battle_bonus_and_lasgun_shield_limit_at_low_threshold: {lo}, resource_amount: {res} }})")
    w(f"    LocationData {{ id: {l['id']}, territory: Territory::from_index({l['territory']}), orientation: {l['orientation']}, sector: {l['sector']}, "
      f"spice_blow_amount: {l['spice']}, discovery_token_type: DiscoveryTokenType::{l['dtt']}, kind: {kind} }},")
w("];")
w("")
w("/// `InitializeLocationNeighbours`: `_locations[a].Neighbours.Add(_locations[b])`, in order.")
w(f"pub(crate) static INITIAL_NEIGHBOURS: [(u8, u8); {len(pairs)}] = [")
for i in range(0, len(pairs), 8):
    w("    " + " ".join(f"({a}, {b_}),"for a, b_ in pairs[i:i + 8]))
w("];")
w("")
w("/// The named properties of `Map` (`Map.PolarSink`, ...).")
w("impl crate::map::Map {")
for name, lid in sorted(names_l.items(), key=lambda kv: kv[1]):
    vis = "pub"
    if name == "HiddenMobileStronghold":
        w(f"    {vis} fn hidden_mobile_stronghold(&self) -> crate::hidden_mobile_stronghold::HiddenMobileStronghold {{ crate::hidden_mobile_stronghold::HiddenMobileStronghold::from_location(Location::from_index({lid})) }}")
    elif any(l["id"] == lid and l["kind"] == "Discovered" for l in locations):
        w(f"    {vis} fn {snake(name)}(&self) -> crate::discovery_stronghold::DiscoveredLocation {{ crate::discovery_stronghold::DiscoveredLocation::from_location(Location::from_index({lid})) }}")
    else:
        w(f"    {vis} fn {snake(name)}(&self) -> Location {{ Location::from_index({lid}) }}")
private = set(re.findall(r"private Territory (\w+) \{", src))
for name, tid in sorted(names_t.items(), key=lambda kv: kv[1]):
    doc = "    /// Private in C#.\n" if name in private else ""
    w(f"{doc}    pub fn {snake(name)}(&self) -> Territory {{ Territory::from_index({tid}) }}")
w("}")
OUT.write_text("\n".join(out) + "\n")
print(f"wrote {OUT}: {len(territories)} territories, {len(locations)} locations, {len(pairs)} neighbour pairs; "
      f"attached-from pairs: {[p for p in pairs if locations[p[0]]['kind'] in ('HiddenMobileStronghold', 'Discovered')]}")
