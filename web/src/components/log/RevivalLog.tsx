// OtherComponents/Log/RevivalLog.razor: one table, a row per revival. Faction pill, then what
// came back (leader disc, Kwisatz Haderach, a token per force), then the cost on the right:
// nothing when free, the card when a card did it, and who paid when someone else did. Other
// lines keep their place: those before the first revival above the table, the rest below it.
// The discard of the card that did a revival is shown by the card in its row, so that line is
// left out.
import { Fragment } from "react";
import type { Faction } from "../../generated/enums";
import type { LogLine } from "../../generated/LogLine";
import { useGame } from "../../state/context";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { SpiceComponent } from "../SpiceComponent";
import { HeroImage, is, PartText, type PhaseLogProps, rec, samePart } from "./common";

export function RevivalLog({ lines, line }: PhaseLogProps) {
  const { skin } = useGame();
  const first = lines.findIndex((l) => is(l, "RevivalRow"));
  const rows = lines.map((l) => rec(l, "RevivalRow")).filter((r) => r != null);
  const messiah = skin.concept("Messiah");
  return (
    <>
      {lines.map((l, i) => {
        if (i === first) {
          return (
            <div key={i} className="log-table revival-table">
              {rows.map((row, r) => {
                const edge = r === 0 ? "first" : "";
                return (
                  <Fragment key={r}>
                    <div className={`cell ${edge}`}><FactionChip faction={row.faction} /></div>
                    <div className={`cell what ${edge}`}>
                      {row.hero?.t === "leader" && <HeroImage hero={row.hero} width={26} className="leader-disc" />}
                      {row.hero?.t === "messiah" && <img className="kh" src={skin.url(skin.str("Messiah_URL"))} alt={messiah} title={messiah} />}
                      {row.asGhola && <span className="note">as Ghola</span>}
                      {Array.from({ length: row.forces }, (_, n) => <ForceToken key={`f${n}`} faction={row.faction} size={18} />)}
                      {Array.from({ length: row.specialForces }, (_, n) => <ForceToken key={`s${n}`} faction={row.faction} special size={18} />)}
                    </div>
                    <div className={`cell cost end ${edge}`}>
                      {row.card != null ? <PartText part={row.card} /> : row.cost > 0 && <SpiceComponent amount={String(row.cost)} />}
                      {row.paidByOther > 0 && (
                        <div className="by">
                          {row.paidByOther < row.cost && <SpiceComponent amount={String(row.paidByOther)} size={16} />}
                          {" paid by "}<Sigil faction={row.payer} />
                        </div>
                      )}
                      {row.tleilaxuIncome > 0 && (
                        <div className="by">→ <Sigil faction="Tleilaxu" /> get <SpiceComponent amount={String(row.tleilaxuIncome)} size={16} /></div>
                      )}
                    </div>
                  </Fragment>
                );
              })}
            </div>
          );
        }
        return !is(l, "RevivalRow") && !discardOfRevivalCard(lines, i) ? <Fragment key={i}>{line(l)}</Fragment> : null;
      })}
    </>
  );
}

// A faction's sigil, small, named on hover.
function Sigil({ faction }: { faction: Faction }) {
  const { skin } = useGame();
  return <img className="sig" src={skin.factionImage(faction)} alt={skin.faction(faction)} title={skin.faction(faction)} />;
}

// The discard straight after a revival done by that card.
function discardOfRevivalCard(lines: LogLine[], i: number): boolean {
  if (i === 0) return false;
  const fact = lines[i].fact;
  const row = rec(lines[i - 1], "RevivalRow");
  return fact?.kind === "Discard" && fact.card != null && row?.card != null && samePart(row.card, fact.card) && row.faction === fact.faction;
}
