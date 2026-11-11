// OtherComponents/Log/BiddingLog.razor: one block per card, with the card number, the winner's
// pill and on the right the price, with "to [sigil]" under it when a faction was paid. A card won
// with Karama shows the card played instead of a price. What follows from the sale hangs under
// the block: an ally's share, the winner's private "You won" line, Harkonnen's extra card. Every
// other line keeps its place.
import { Fragment, type ReactNode } from "react";
import type { LogLine } from "../../generated/LogLine";
import { useGame } from "../../state/context";
import { FactionChip } from "../FactionChip";
import { RichText } from "../RichText";
import { SpiceComponent } from "../SpiceComponent";
import { is, PartText, type PhaseLogProps, type RecordOf } from "./common";

// A card's block: the sale (or, when everyone passed, just the number) and the lines under it.
interface Lot { kind: "lot"; number: number; sold: RecordOf<"CardSold"> | null; under: LogLine[] }
type Item = Lot | { kind: "line"; line: LogLine };

function items(lines: LogLine[]): Item[] {
  const result: Item[] = [];
  let current: Lot | null = null;
  for (const l of lines) {
    const r = l.record;
    if (r?.type === "CardSold") {
      current = { kind: "lot", number: r.cardNumber, sold: r, under: [] };
      result.push(current);
    } else if ((r?.type === "CardWonShown" || r?.type === "ExtraCardReceived" || r?.type === "ExtraCardShown") && current != null) {
      current.under.push(l);
    } else if (r?.type === "BiddingPassedByAll") {
      result.push({ kind: "lot", number: r.cardNumber, sold: null, under: [] });
      result.push({ kind: "line", line: l });
      current = null;
    } else {
      result.push({ kind: "line", line: l });
    }
  }
  return result;
}

export function BiddingLog({ lines, line }: PhaseLogProps) {
  return (
    <>
      {items(lines).map((item, i): ReactNode => {
        if (item.kind === "lot") return <Block key={i} lot={item} />;
        if (is(item.line, "BiddingDrawn", "TreacheryReshuffled", "BiddingPassedByAll")) {
          return <div key={i} className="bid-note"><RichText parts={item.line.text} size={20} logStyle /></div>;
        }
        return <Fragment key={i}>{line(item.line)}</Fragment>;
      })}
    </>
  );
}

function Block({ lot }: { lot: Lot }) {
  const { skin } = useGame();
  const sold = lot.sold;
  return (
    <div className={`bid-lot ${sold == null ? "unsold" : ""}`}>
      <div className="top">
        <span className="n">Card {lot.number}</span>
        {sold && (
          <>
            <FactionChip faction={sold.winner} />
            <span className="price">
              {sold.karamaCard != null ? <PartText part={sold.karamaCard} /> : (
                <>
                  <SpiceComponent amount={String(sold.price)} size={22} />
                  {sold.paidTo !== "None" && (
                    <span className="payee">to <img src={skin.factionImage(sold.paidTo)} alt={skin.faction(sold.paidTo)} title={skin.faction(sold.paidTo)} /></span>
                  )}
                </>
              )}
            </span>
          </>
        )}
      </div>
      {((sold != null && sold.shares.length > 0) || lot.under.length > 0) && (
        <div className="under">
          {(sold?.shares ?? []).map((s, i) => <div key={`s${i}`}><SpiceComponent amount={String(s.amount)} /> paid by <FactionChip faction={s.by} /></div>)}
          {lot.under.map((l, i) => <div key={`u${i}`} className={l.private ? "private" : ""}><RichText parts={l.text} size={20} logStyle /></div>)}
        </div>
      )}
    </div>
  );
}
