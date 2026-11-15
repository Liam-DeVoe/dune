"""Converts an `observe record` trace (op lines) into the recorded-game format the view and skin
tests read (kind lines, see `Trace` in rust/view/tests/common/mod.rs). The short games in
rust/view/tests/data/battle and data/other were made this way, from games recorded with
`observe record --mode bots|fuzz --seed S --game I --out FILE`.

Usage: record-to-test-trace.py IN.jsonl.gz OUT.jsonl.gz [CUT]
CUT: stop after the CUT-th accepted event."""
import gzip, json, sys

src, dst = sys.argv[1], sys.argv[2]
cut = int(sys.argv[3]) if len(sys.argv) > 3 else None
out = []
n = 0
accepted = 0
for line in gzip.open(src, 'rt'):
    o = json.loads(line)
    op = o['op']
    if op == 'header':
        seated = o.get('seated') or []
        part = {"$type": "Participation", "AvailableSeats": [], "BotsSpeed": 1, "Hosts": [], "Kicked": [], "Observers": [],
                "PlayerNames": [[u, name] for u, s, name in seated], "SeatedPlayers": [[u, s] for u, s, name in seated]}
        out.append({"kind": "header", "traceFormat": 2, "mode": o['mode'], "masterSeed": o['seed'], "gameIndex": o['index'],
                    "gameVersion": o['version'], "config": o.get('config'), "participation": part, "recorder": "observe record (converted)"})
    elif op == 'event':
        if cut is not None and accepted >= cut:
            break
        result = o['result']
        if result not in ('accepted', 'rejected'):
            result = 'error'
        out.append({"kind": "step", "n": n, "source": o.get('src'), "actor": o.get('p'), "validate": True, "isHost": o.get('host', False),
                    "queries": [], "event": o['event'], "result": result})
        n += 1
        if result == 'accepted':
            accepted += 1
    elif op in ('undo', 'restore', 'dismiss'):
        if cut is not None and accepted >= cut:
            break
        kind = {'undo': 'undo', 'restore': 'restoreRecentlyUndone', 'dismiss': 'dismissRecentlyUndone'}[op]
        l = {"kind": kind, "n": n, "queries": []}
        if op == 'undo':
            l["untilEventNr"] = o['until']
        out.append(l)
        n += 1
    elif op == 'end':
        out.append({"kind": "end", "queries": [], "reason": o['reason'] if cut is None else 'cut'})
if out[-1]['kind'] != 'end':
    out.append({"kind": "end", "queries": [], "reason": "cut"})
with gzip.open(dst, 'wt') as f:
    for l in out:
        f.write(json.dumps(l, separators=(',', ':')) + '\n')
