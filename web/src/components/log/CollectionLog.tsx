// OtherComponents/Log/CollectionLog.razor: one table of who, how much and where. Spice collected
// from the desert comes first; stronghold and homeworld income follows under a brighter rule.
// When two fields in one territory paid out, each row names its sector under the place. Other
// lines keep their place: those before the first collection above the table, the rest below it.
import { Fragment } from "react";
import { useGame } from "../../state/context";
import { FactionChip } from "../FactionChip";
import { SpiceComponent } from "../SpiceComponent";
import { is, type PhaseLogProps, PlacePart, placeId, rec, type RecordOf } from "./common";

type Collected = RecordOf<"SpiceCollected">;

export function CollectionLog({ lines, line }: PhaseLogProps) {
  const first = lines.findIndex((l) => is(l, "SpiceCollected"));
  const collected = lines.map((l) => rec(l, "SpiceCollected")).filter((c) => c != null);
  const fields = collected.filter((c) => !c.income);
  const income = collected.filter((c) => c.income);
  // Territories where more than one field paid out.
  const fieldsByTerritory = new Map<number, Set<number>>();
  for (const c of fields) {
    if (c.field == null) continue;
    const t = placeId(c.territory);
    if (!fieldsByTerritory.has(t)) fieldsByTerritory.set(t, new Set());
    fieldsByTerritory.get(t)!.add(placeId(c.field));
  }
  const split = new Set([...fieldsByTerritory].filter(([, f]) => f.size > 1).map(([t]) => t));
  return (
    <>
      {lines.map((l, i) => {
        if (i === first) {
          return (
            <div key={i} className="log-table collection-table">
              {fields.map((c, r) => <Row key={`f${r}`} c={c} first={r === 0} showSector={split.has(placeId(c.territory))} />)}
              {fields.length > 0 && income.length > 0 && <div className="income-rule"></div>}
              {income.map((c, r) => <Row key={`i${r}`} c={c} first={r === 0} showSector={false} />)}
            </div>
          );
        }
        return is(l, "SpiceCollected") ? null : <Fragment key={i}>{line(l)}</Fragment>;
      })}
    </>
  );
}

function Row({ c, first, showSector }: { c: Collected; first: boolean; showSector: boolean }) {
  const { skin } = useGame();
  const edge = first ? "first" : "";
  const field = c.field != null ? skin.locationInfo(placeId(c.field)) : undefined;
  return (
    <>
      <div className={`cell faction ${edge}`}><FactionChip faction={c.faction} /></div>
      <div className={`cell amount ${edge}`}><SpiceComponent amount={String(c.amount)} /></div>
      <div className={`cell place ${edge}`}>
        <PlacePart part={c.territory} />
        {showSector && field && (
          <div className="log-label sector">{field.orientation !== "" ? `${field.orientation} sector` : `sector ${field.sector}`}</div>
        )}
      </div>
    </>
  );
}
