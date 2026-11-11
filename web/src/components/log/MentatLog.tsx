// OtherComponents/Log/MentatLog.razor: bribes collected, one row per faction; when the game ends,
// a result block with the winners and the game's reason line. If a Bene Gesserit prediction took
// over the win, the earlier reason stays a plain line where it was. Other lines keep their place.
import { Fragment } from "react";
import { FactionChip } from "../FactionChip";
import { RichText } from "../RichText";
import { SpiceComponent } from "../SpiceComponent";
import { is, type PhaseLogProps, plainText, rec } from "./common";

export function MentatLog({ lines, line }: PhaseLogProps) {
  const firstBribe = lines.findIndex((l) => is(l, "BribesCollected"));
  const ended = lines.findIndex((l) => is(l, "GameEnded"));
  // FindLastIndex(ended, ...): the last reason at or before the end.
  let reason = -1;
  if (ended >= 0) for (let i = ended; i >= 0; i--) if (is(lines[i], "WinReason")) { reason = i; break; }
  const bribes = lines.map((l) => rec(l, "BribesCollected")).filter((b) => b != null);
  return (
    <>
      {lines.map((l, i) => {
        const result = rec(l, "GameEnded");
        if (i === firstBribe) {
          return (
            <Fragment key={i}>
              <div className="log-label mentat-bribes-label">Bribes collected</div>
              <div className="log-cols mentat-bribes">
                {bribes.map((b, j) => (
                  <Fragment key={j}>
                    <div><FactionChip faction={b.faction} /></div>
                    <div><SpiceComponent amount={String(b.amount)} /></div>
                  </Fragment>
                ))}
              </div>
            </Fragment>
          );
        }
        if (i === ended && result) {
          return (
            <div key={i} className="mentat-result">
              <div className="title">{plainText(l.text)}</div>
              {result.winners.length > 0 && (
                <div className="winners">
                  {result.winners.map((w, j) => <span key={j} className="winner"><FactionChip faction={w} /> win!</span>)}
                </div>
              )}
              {reason >= 0 && <div className="reason"><RichText parts={lines[reason].text} size={20} logStyle /></div>}
            </div>
          );
        }
        if (!is(l, "BribesCollected", "GameWinner") && i !== reason) return <Fragment key={i}>{line(l)}</Fragment>;
        return null;
      })}
    </>
  );
}
