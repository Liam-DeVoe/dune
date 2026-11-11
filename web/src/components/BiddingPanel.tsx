// OtherComponents/BiddingPanel.razor with NormalBiddingPanel.razor (Bid, Phase.Bidding) and
// BlackMarketBiddingPanel.razor (BlackMarketBid, Phase.BlackMarketBidding). ActionPanel draws it
// by phase for every viewer: the lot (view.bidding.panel) for all, the inputs (the Bid or
// BlackMarketBid action) for the bidder, and auto-pass for players.
//
// Like the C# panel, the inputs restart at the lowest winning bid after every event outside
// silent auctions (OnParametersSetAsync), and auto-pass and the bidding timer pass for the player.
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { ActionOptions } from "../generated/ActionOptions";
import type { ActionView } from "../generated/ActionView";
import type { BiddingPanelView } from "../generated/BiddingPanelView";
import type { DraftResult } from "../generated/DraftResult";
import type { Faction } from "../generated/enums";
import { factionChipHtml, treacheryCardPopup } from "../popups";
import { usePopover, popoverAttributes } from "../popover";
import { useGame } from "../state/context";
import { ButtonRow } from "./ButtonRow";
import { Checkbox } from "./Checkbox";
import { FactionChip } from "./FactionChip";
import { Image } from "./Image";
import { SelectResources, SelectResourcesFromFaction } from "./NumberPicker";
import { SelectFromImage } from "./SelectFromImage";
import { SpiceComponent } from "./SpiceComponent";
import { useViewChanges } from "./formDraft";

// Client.AutoPass, BidAutoPassThreshold and KeepAutoPassSetting: client state that outlives the
// panel (it is remounted for every auction) but not the page, as in the C# Client.
interface AutoPassState { autoPass: boolean; threshold: number; keep: boolean }
let autoPassState: AutoPassState = { autoPass: false, threshold: 0, keep: false };
const listeners = new Set<() => void>();
const autoPassStore = {
  get: () => autoPassState,
  set(changes: Partial<AutoPassState>) {
    autoPassState = { ...autoPassState, ...changes };
    listeners.forEach((l) => l());
  },
  subscribe(l: () => void) {
    listeners.add(l);
    return () => { listeners.delete(l); };
  },
};

// Client.ResetAutoPassThreshold, after every event in the bidding main phase: auto-pass ends
// with the auction unless it is to repeat, and always with the bidding.
export function useAutoPassReset() {
  const { view } = useGame();
  useEffect(() => {
    if (view.mainPhase === "Bidding" && view.bidding?.auctionWon && (!autoPassStore.get().keep || view.phase === "BiddingReport")) {
      autoPassStore.set({ autoPass: false });
    }
  }, [view]);
}

type BidOptions = Extract<ActionOptions, { form: "Bid" } | { form: "BlackMarketBid" }>;

interface Draft { amount: number; ally: number; emperor: number; karmaCard: number | null; secretAlly: boolean }

// InfoComponent.razor: the info icon with a popover.
function Info({ contents }: { contents: string }) {
  const html = `<div class='pop'>${contents}</div>`;
  const ref = usePopover<HTMLDivElement>(html);
  return (
    <div ref={ref} className="d-inline" {...popoverAttributes(html, "image")}>
      <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} fill="var(--ink-0)" className="bi bi-info-circle" viewBox="0 0 16 16">
        <circle cx="8" cy="8" r="8" fill="var(--gold)" />
        <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z" />
        <path d="M8.93 6.588l-2.29.287-.082.38.45.083c.294.07.352.176.288.469l-.738 3.468c-.194.897.105 1.319.808 1.319.545 0 1.178-.252 1.465-.598l.088-.416c-.2.176-.492.246-.686.246-.275 0-.375-.193-.304-.533L8.93 6.588zM9 4.5a1 1 0 1 1-2 0 1 1 0 0 1 2 0z" />
      </svg>
    </div>
  );
}

function Lot({ panel }: { panel: BiddingPanelView }) {
  const { view, skin } = useGame();
  const auctionType = view.bidding?.auctionType ?? "Normal";
  // NormalBiddingPanel.AuctionDescription / BlackMarketBiddingPanel's "Black Market".
  const title = panel.blackMarket
    ? "Black Market"
    : panel.cardOf
      ? `Card ${panel.cardOf.number} of ${panel.cardOf.of}`
      : `${factionChipHtml(skin, "Richese", view.viewer.faction === "Richese")} Card (${skin.enumValue("AuctionType", auctionType)})`;
  return (
    <div className="lot">
      {panel.card != null
        ? <Image src={skin.cardImage(panel.card)} alt={skin.card(panel.card)} width={64} popover={treacheryCardPopup(skin, panel.card)} />
        : <img src={skin.url(skin.str("CardBack_TreacheryCard_URL"))} width={64} alt="Card on sale" />}
      <div className="lot-text">
        <div className="lot-title" dangerouslySetInnerHTML={{ __html: title }} />
        {panel.currentBid
          ? <div className="lot-bid"><FactionChip faction={panel.currentBid.faction as Faction} /> <SpiceComponent amount={String(panel.currentBid.amount)} /></div>
          : !panel.silent && <div className="lot-bid">No bids yet</div>}
      </div>
    </div>
  );
}

function defaults(o: BidOptions | null): Pick<Draft, "amount" | "ally"> {
  return { amount: o?.defaultAmount ?? 0, ally: o?.defaultAllyAmount ?? 0 };
}

function toEvent(eventType: string, d: Draft): Record<string, unknown> {
  const amounts = { Amount: d.amount, AllyContributionAmount: d.ally, EmperorContributionAmount: d.emperor };
  return eventType === "Bid"
    ? { ...amounts, UsesEmperorSecretAlly: d.secretAlly, KarmaBid: false, _karmaCardId: d.karmaCard ?? -1, Passed: false }
    : { ...amounts, Passed: false };
}

// The bidder's inputs and buttons (BiddingPanel.razor:41-107).
function BidInputs({ action, panel }: { action: ActionView; panel: BiddingPanelView }) {
  const { view, skin, source } = useGame();
  const viewOptions = action.options?.form === "Bid" || action.options?.form === "BlackMarketBid" ? action.options : null;
  const [draft, setDraft] = useState<Draft>({ ...defaults(viewOptions), emperor: 0, karmaCard: null, secretAlly: false });
  const [result, setResult] = useState<DraftResult>({ eventType: action.eventType, options: action.options, validation: action.validation });
  const latest = useRef(0);

  const ask = async (next: Draft) => {
    const n = ++latest.current;
    const r = await source.query({ $type: action.eventType, Initiator: 0, ...toEvent(action.eventType, next) });
    if (n === latest.current) setResult(r);
  };
  const current = useRef(draft);
  current.current = draft;

  // OnParametersSetAsync after each event: outside silent auctions the amounts restart at the
  // lowest winning bid. The other inputs are kept, as the component's fields are, and the draft is
  // validated again (formDraft.ts).
  const seen = useRef(view.eventNumber);
  useViewChanges(() => {
    const event = seen.current !== view.eventNumber;
    seen.current = view.eventNumber;
    setResult({ eventType: action.eventType, options: action.options, validation: action.validation });
    const next = panel.silent || !event ? current.current : { ...current.current, ...defaults(viewOptions) };
    setDraft(next);
    void ask(next);
  });

  const update = (changes: Partial<Draft>) => {
    const next = { ...draft, ...changes };
    setDraft(next);
    void ask(next);
  };
  const send = (event: Record<string, unknown>) => source.submit({ $type: action.eventType, Initiator: 0, ...event, eventNumber: view.eventNumber });

  const o = result.options?.form === "Bid" || result.options?.form === "BlackMarketBid" ? result.options : viewOptions;
  if (!o) return null;
  const karmaCards = o.form === "Bid" ? o.karmaCards : [];
  const canUseKarma = karmaCards.length > 0;
  const canUseSecretAlly = o.form === "Bid" && o.mayUseEmperorSecretAlly;
  const karmaCard = canUseKarma ? draft.karmaCard : null;
  const secretAlly = canUseSecretAlly && draft.secretAlly;
  const ally = view.players.find((p) => p.faction === view.viewer.faction)?.ally ?? "None";
  const allyPicker = ally !== "None" && o.maxAllyAmount > 0;
  const allyAmount = allyPicker ? draft.ally : 0;
  const karmaName = skin.enumValue("TreacheryCardType", "Karma");
  const total = draft.amount + allyAmount + draft.emperor;
  const bidLabel = `Bid ${total}${karmaCard != null ? ` (with ${karmaName})` : ""}`;
  const karmaInfo = `If you use a ${karmaName} card to bid any amount of ${skin.concept("Resource")} you wish, that card will be set aside until someone outbids you and will be discarded only if you win the auction. If you instead use your ${karmaName} card to win immediately, it is immediately discarded.`;

  const confirm = () => send(toEvent(action.eventType, { ...draft, ally: allyAmount, karmaCard, secretAlly }));
  const pass = () => send(action.eventType === "Bid" ? { Passed: true, KarmaBid: false } : { Passed: true });
  const other = () => send({ Passed: false, KarmaBid: true, _karmaCardId: karmaCard ?? -1 });

  return (
    <>
      {canUseSecretAlly && (
        <Checkbox value={secretAlly} onChange={(v) => update({ secretAlly: v })}>Use your Nexus card to get this card for free?</Checkbox>
      )}
      {canUseKarma && (
        <div className="mt-1">
          <SelectFromImage
            values={karmaCards.map((c) => ({ value: c, src: skin.cardImage(c), alt: skin.card(c), popover: treacheryCardPopup(skin, c) }))}
            value={karmaCard} onChange={(c) => update({ karmaCard: c })} required={false} imageWidth={80}
            label={<>Use {karmaName} to bid without limit or win immediately? <Info contents={karmaInfo} /></>} />
        </div>
      )}
      <div className="mt-1">
        {allyPicker
          ? <>
              <SelectResourcesFromFaction value={draft.amount} onChange={(v) => update({ amount: v })} min={0} max={o.maxAmount} faction={view.viewer.faction as Faction} />
              <SelectResourcesFromFaction value={draft.ally} onChange={(v) => update({ ally: v })} min={0} max={o.maxAllyAmount} faction={ally as Faction} />
            </>
          : <SelectResources value={draft.amount} onChange={(v) => update({ amount: v })} min={0} max={o.maxAmount} />}
        {o.maxEmperorAmount > 0 && (
          <SelectResourcesFromFaction value={draft.emperor} onChange={(v) => update({ emperor: v })} min={0} max={o.maxEmperorAmount} faction="Emperor" />
        )}
      </div>
      <ButtonRow confirm={confirm} confirmText={bidLabel} confirmError={result.validation}
                 pass={panel.silent ? undefined : pass}
                 other={karmaCard != null ? other : undefined} otherText="Win immediately"
                 alignment="justify-content-start" />
    </>
  );
}

// The auto-pass setting (BiddingPanel.razor:109-123), and passing for the player: by auto-pass
// when it becomes their turn and the bid to beat is above the threshold, after a short delay
// (OnParametersSetAsync), and when the bidding timer runs out (CycleAsync).
function AutoPass({ action, panel }: { action: ActionView | undefined; panel: BiddingPanelView }) {
  const { view, source } = useGame();
  const state = useSyncExternalStore(autoPassStore.subscribe, autoPassStore.get);
  const canBid = action !== undefined;
  const pass = () => {
    if (!action) return;
    source.submit({ $type: action.eventType, Initiator: 0, Passed: true, ...(action.eventType === "Bid" ? { KarmaBid: false } : {}), eventNumber: view.eventNumber });
  };
  const passRef = useRef(pass);
  passRef.current = pass;

  const couldBid = useRef(false);
  useEffect(() => {
    if (!couldBid.current && canBid) {
      const s = autoPassStore.get();
      const bid = panel.currentBid;
      if ((bid != null && s.autoPass && bid.amount + 1 > s.threshold) || (bid == null && s.autoPass && s.threshold === 0)) {
        const t = window.setTimeout(() => { if (autoPassStore.get().autoPass) passRef.current(); }, 800 + Math.floor(Math.random() * 1600));
        couldBid.current = canBid;
        return () => window.clearTimeout(t);
      }
    }
    couldBid.current = canBid;
  }, [canBid, view.eventNumber, panel.currentBid]);

  useEffect(() => {
    const timer = view.timing.timer;
    const since = view.status.timedEventTime;
    if (!canBid || timer <= 0 || since == null) return;
    const id = window.setInterval(() => {
      if ((Date.now() - Date.parse(since)) / 1000 > timer) passRef.current();
    }, 1000);
    return () => window.clearInterval(id);
  }, [canBid, view.timing.timer, view.status.timedEventTime]);

  if (view.viewer.isObserver || view.viewer.faction == null || view.viewer.faction === "None") return null;
  const resources = view.me?.resources ?? 0;
  return (
    <div className="autopass">
      <Checkbox value={state.autoPass} onChange={(v) => {
        // AutoPassChanged: a threshold above one's spice starts at min(spice, 5).
        autoPassStore.set(v && state.threshold > resources ? { autoPass: v, threshold: Math.min(resources, 5) } : { autoPass: v });
      }}>Auto-pass</Checkbox>
      {state.autoPass && (
        <>
          <SelectResources value={state.threshold} onChange={(v) => autoPassStore.set({ threshold: v })} min={0} max={50} label="When above:" />
          {!panel.blackMarket && (
            <Checkbox value={state.keep} onChange={(v) => autoPassStore.set({ keep: v })}>Repeat auto-pass every round</Checkbox>
          )}
        </>
      )}
    </div>
  );
}

export function BiddingPanel({ panel }: { panel: BiddingPanelView }) {
  const { view } = useGame();
  const action = view.actions.find((a) => a.eventType === (panel.blackMarket ? "BlackMarketBid" : "Bid"));
  return (
    <div className="card p-1 mb-2 bidding-panel" style={{ animation: "fadeIn 1s" }}>
      <div className="card-body">
        <Lot panel={panel} />
        {action && <BidInputs key={`${action.eventType}`} action={action} panel={panel} />}
        <AutoPass action={action} panel={panel} />
      </div>
    </div>
  );
}
