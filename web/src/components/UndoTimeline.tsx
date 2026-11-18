// The timeline under the map while the host chooses a moment
// to undo to. The top strip is the whole game, one block per turn; the track under it is the
// focused turn, one segment per phase and one tick per event. The handle always sits in a gap
// between two events. Every gap has an invisible zone around it, so pressing and dragging across
// the track picks gaps without measuring the track. Hovering a gap shows the event it keeps and
// the one it undoes.
import { useState } from "react";
import { useGame } from "../state/context";
import type { UndoEntry } from "../state/source";
import { RichText } from "./RichText";
import { listedEvents, undoSession, useUndoSession } from "./undoSession";

// A percentage with at most 3 decimals.
const pct = (part: number, whole: number) => `${Number((whole === 0 ? 0 : (100 * part) / whole).toFixed(3))}%`;

export function UndoTimeline() {
  const { skin } = useGame();
  const session = useUndoSession();
  const [focusTurn, setFocus] = useState<number | null>(null);
  const [hover, setHover] = useState<number | null>(null);
  const entries = session?.entries;
  if (!session || !entries) return null;
  const events = listedEvents(entries);
  if (events.length === 0) return null;

  const at = (n: number): UndoEntry => entries[n];
  const phase = (p: string) => skin.enumValue("MainPhase", p);
  const now = entries.length;
  const atNow = session.gap === null;
  const cutIndex = atNow ? events.length : events.findIndex((i) => i >= session.gap!);
  const turns: { turn: number; events: number[] }[] = [];
  for (const e of events) {
    const t = at(e).turn;
    if (turns.length && turns[turns.length - 1].turn === t) turns[turns.length - 1].events.push(e);
    else if (turns.some((x) => x.turn === t)) turns.find((x) => x.turn === t)!.events.push(e);
    else turns.push({ turn: t, events: [e] });
  }
  const turnOfCut = at(events[Math.min(cutIndex, events.length - 1)]).turn;
  const focus = turns.find((t) => t.turn === (focusTurn ?? turnOfCut)) ?? turns[turns.length - 1];
  const first = events.indexOf(focus.events[0]);
  const m = focus.events.length;
  const isLast = focus.turn === turns[turns.length - 1].turn;
  const cutInTurn = cutIndex - first;

  // Runs of consecutive events in the same phase, as [from, to) positions within the turn.
  const phases: [number, number, string][] = [];
  let from = 0;
  for (let j = 1; j <= m; j++) {
    if (j === m || at(focus.events[j]).phase !== at(focus.events[from]).phase) {
      phases.push([from, j, at(focus.events[from]).phase]);
      from = j;
    }
  }

  const kept = hover !== null && first + hover - 1 >= 0 ? events[first + hover - 1] : null;
  const undone = hover !== null && first + hover < events.length ? events[first + hover] : null;

  return (
    <div className="undo-timeline">
      <div className="undo-timeline__turns">
        {turns.map((t) => (
          <div key={t.turn} className={`undo-timeline__turn ${t.turn === focus.turn ? "undo-timeline__turn--focus" : ""}`} style={{ flex: t.events.length }}
               onClick={() => { setFocus(t.turn); setHover(null); }} title={`Turn ${t.turn}`}>{t.turn}</div>
        ))}
        {!atNow && (
          <>
            <div className="undo-timeline__gone" style={{ left: pct(cutIndex, events.length) }}></div>
            <div className="undo-timeline__mark" style={{ left: pct(cutIndex, events.length) }}></div>
          </>
        )}
      </div>

      <div className="undo-timeline__track" onPointerLeave={() => setHover(null)}>
        {phases.map(([f, t, p]) => (
          <div key={f} className="undo-timeline__phase" style={{ left: pct(f, m), width: pct(t - f, m) }} title={phase(p)}>
            <span className="undo-timeline__phase-name">{phase(p)}</span>
          </div>
        ))}
        {Array.from({ length: m }, (_, j) => <div key={j} className="undo-timeline__tick" style={{ left: pct(j * 2 + 1, m * 2) }}></div>)}
        {!atNow && cutInTurn < m && <div className="undo-timeline__gone" style={{ left: pct(Math.max(0, cutInTurn), m) }}></div>}
        {isLast && <div className="undo-timeline__now"></div>}
        {cutInTurn >= 0 && (cutInTurn < m || isLast) && <div className="undo-timeline__handle" style={{ left: pct(cutInTurn, m) }}></div>}
        {Array.from({ length: m + 1 }, (_, j) => {
          const gap = first + j < events.length ? events[first + j] : now;
          return (
            <div key={`z${j}`} className="undo-timeline__zone"
                 style={{ left: pct(Math.max(0, j * 2 - 1), m * 2), right: pct(Math.max(0, m * 2 - j * 2 - 1), m * 2) }}
                 onPointerDown={() => undoSession.moveTo(gap)}
                 onPointerEnter={(e) => { setHover(j); if ((e.buttons & 1) !== 0) undoSession.moveTo(gap); }}></div>
          );
        })}
        {hover !== null && hover <= m && (
          <div className="pop undo-tip" style={{ left: `clamp(170px, ${pct(hover, m)}, calc(100% - 170px))` }}>
            <div className="undo-tip__when">{undone !== null ? `Turn ${at(undone).turn} · ${phase(at(undone).phase)}` : "Now"}</div>
            {kept !== null && <div className="undo-tip__line"><RichText parts={at(kept).text} size={18} /></div>}
            <div className="undo-tip__cut"></div>
            {undone !== null
              ? <div className="undo-tip__line undo-tip__line--out"><RichText parts={at(undone).text} size={18} /></div>
              : <div className="undo-tip__line undo-tip__line--nothing">Nothing is undone</div>}
          </div>
        )}
      </div>
    </div>
  );
}
