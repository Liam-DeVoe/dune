// OtherComponents/DealToasts.razor: a toast above the Deals icon for each deal offered to the
// viewer. It shows the game's own log line for the offer and goes after five seconds, or stays
// while the mouse is over it. Clicking it opens Deals with the offer marked; × closes it and leaves
// the offer waiting. Answering the offer removes it. If the offer leaves unanswered, it says
// "Offer withdrawn" for two seconds, then goes. Offers already there when the game loads get no
// toast.
//
// C# tells a withdrawn offer from an answered one by the game's deal responses. The view has
// only the open offers, so an offer that goes counts as answered when this client answered it
// (answeredDeals, which DealComponent fills).
import { useEffect, useRef, useState } from "react";
import type { DealOfferView } from "../generated/DealOfferView";
import { useGame } from "../state/context";
import { menu } from "./menuState";
import { RichText } from "./RichText";

const LIFETIME = 5000;
const WITHDRAWN_LIFETIME = 2000;
const TICK = 100;

// The offers this client answered, by id.
export const answeredDeals = new Set<number>();

interface Toast {
  offer: DealOfferView;
  left: number;
  hovered: boolean;
  withdrawn: boolean;
}

export function DealToasts() {
  const { view, skin } = useGame();
  const [toasts, setToasts] = useState<Toast[]>([]);
  const seen = useRef<Set<number> | null>(null);
  const offers = view.deals?.acceptable ?? [];
  const ids = offers.map((o) => o.id).join(",");

  // Compares the offers made to the viewer with the ones already seen.
  useEffect(() => {
    if (seen.current === null) {
      seen.current = new Set(offers.map((o) => o.id));
      return;
    }
    const s = seen.current;
    setToasts((ts) => {
      let next = [...ts];
      for (const o of offers) {
        if (!s.has(o.id)) {
          s.add(o.id);
          next.push({ offer: o, left: LIFETIME, hovered: false, withdrawn: false });
        }
      }
      next = next.flatMap((t) => {
        if (t.withdrawn || offers.some((o) => o.id === t.offer.id)) return [t];
        return answeredDeals.has(t.offer.id) ? [] : [{ ...t, withdrawn: true, left: WITHDRAWN_LIFETIME }];
      });
      // Three at most; the oldest makes room.
      return next.slice(Math.max(0, next.length - 3));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ids]);

  const running = toasts.length > 0;
  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => {
      setToasts((ts) => ts
        .map((t) => (t.hovered && !t.withdrawn ? t : { ...t, left: t.left - TICK }))
        .filter((t) => t.left > 0));
    }, TICK);
    return () => window.clearInterval(timer);
  }, [running]);

  if (toasts.length === 0) return null;
  const update = (id: number, change: Partial<Toast>) => setToasts((ts) => ts.map((t) => (t.offer.id === id ? { ...t, ...change } : t)));
  const remove = (id: number) => setToasts((ts) => ts.filter((t) => t.offer.id !== id));
  const open = (t: Toast) => {
    if (t.withdrawn) return;
    remove(t.offer.id);
    menu.markDeal(t.offer.id);
    menu.open("Deals");
  };
  return (
    <div className="deal-toasts">
      {toasts.map((t) => (
        <div key={t.offer.id} className={`deal-toast ${t.withdrawn ? "gone" : ""}`}
             onMouseEnter={() => update(t.offer.id, { hovered: true })} onMouseLeave={() => update(t.offer.id, { hovered: false })}
             onClick={() => open(t)}>
          <img className="sigil-lg" src={skin.factionImage(t.offer.initiator)} alt={skin.faction(t.offer.initiator)} />
          <div className="txt">
            <RichText parts={t.offer.message} />
            {t.withdrawn && <span className="withdrawn">Offer withdrawn</span>}
          </div>
          <button className="x" title="Close" onClick={(e) => { e.stopPropagation(); remove(t.offer.id); }}>×</button>
        </div>
      ))}
    </div>
  );
}
