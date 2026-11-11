// OtherComponents/Log/CharityLog.razor: one row per claim, faction pills down the left and the
// amounts down the right. Other lines keep their place: those before the first claim above the
// table, the rest below it.
import { Fragment } from "react";
import { FactionChip } from "../FactionChip";
import { SpiceComponent } from "../SpiceComponent";
import { is, type PhaseLogProps, rec } from "./common";

export function CharityLog({ lines, line }: PhaseLogProps) {
  const first = lines.findIndex((l) => is(l, "CharityClaim"));
  const claims = lines.map((l) => rec(l, "CharityClaim")).filter((c) => c != null);
  return (
    <>
      {lines.map((l, i) => {
        if (i === first) {
          return (
            <div key={i} className="log-cols">
              {claims.map((c, j) => (
                <Fragment key={j}>
                  <div><FactionChip faction={c.faction} /></div>
                  <div><SpiceComponent amount={String(c.amount)} /></div>
                </Fragment>
              ))}
            </div>
          );
        }
        return is(l, "CharityClaim") ? null : <Fragment key={i}>{line(l)}</Fragment>;
      })}
    </>
  );
}
