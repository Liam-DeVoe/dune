// OtherComponents/UndoComponent.razor: the host's undo panel. Opening it starts an undo session:
// the host drags the timeline under the map (UndoTimeline) to a moment. The panel says where the
// game goes back to and lists what would be undone. Escape cancels.
import { Fragment, useEffect } from "react";
import { useGame } from "../state/context";
import { RichText } from "./RichText";
import { undoSession, useUndoSession } from "./undoSession";

export function UndoComponent({ onClose }: { onClose: () => void }) {
  const { skin, source } = useGame();
  const session = useUndoSession();

  // Client.OpenUndo on first render, Client.CloseUndo when the panel goes.
  useEffect(() => {
    undoSession.open(() => source.undoList?.() ?? Promise.resolve([]), source.undoPreview?.bind(source));
    return () => undoSession.close();
  }, [source]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  if (!session) return null;
  const entries = session.entries ?? [];
  const atNow = session.gap === null;
  const gap = session.gap ?? entries.length;
  const count = entries.length - gap;
  const phase = (p: string) => skin.enumValue("MainPhase", p);

  const confirm = async () => {
    if (session.gap === null) return;
    const g = session.gap;
    undoSession.close();
    await source.gameCall?.("RequestUndo", g);
    onClose();
  };

  const undone = [];
  let group: string | null = null;
  for (let i = entries.length - 1; i >= gap; i--) {
    const e = entries[i];
    if (!e.listed) continue;
    const key = `${e.turn}/${e.phase}`;
    undone.push(
      <Fragment key={i}>
        {group !== key && <div className="group">Turn {e.turn} · {phase(e.phase)}</div>}
        <div className="line"><RichText parts={e.text} size={18} /></div>
      </Fragment>,
    );
    group = key;
  }
  const moment = entries[gap];

  return (
    <div className="panel undo mb-2">
      <header>
        <h5>Undo</h5>
        <button className="btn-close btn-close-sm" title="Cancel" onClick={onClose}></button>
      </header>
      <div className="body">
        {atNow || !moment
          ? <div className="empty">Drag the timeline under the map back to the moment you want to return to.</div>
          : (
            <>
              <div>
                <div className="kicker">Back to</div>
                <div className="dest">Turn {moment.turn} · {phase(moment.phase)}</div>
              </div>
              <div>
                <div className="kicker">Undone</div>
                <div className="undone">{undone}</div>
              </div>
            </>
          )}
        <div className="actions">
          <button className="btn btn-secondary" onClick={onClose}>Cancel</button>{" "}
          <button className="btn btn-primary" disabled={atNow} onClick={confirm}>
            {atNow ? "Undo" : count === 1 ? "Undo 1 event" : `Undo ${count} events`}
          </button>
        </div>
      </div>
    </div>
  );
}
