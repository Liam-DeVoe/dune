// Why a pause between phases waits for a player (PhaseControlInfo.reasons, from
// rust/server/src/pause_rules.rs), in words for the status card.
//
// Other players read "Waiting in case <faction> ..." for each player the pause waits for. That says
// the game stops for a possibility, never what anyone holds: some reasons need a secret card or
// spice, and the pause waits regardless so that waiting gives nothing away.
//
// The waited player's own card can be truthful, since only they see it. Their view's actions say
// whether they can act on a reason. If they can, it reads "You may still ..."; if they can't (no
// Karama, too little spice, ...), it says so, so they don't look for an action that isn't there.
import type { ReactNode } from "react";
import type { Faction } from "../generated/enums";
import type { GameView } from "../generated/GameView";
import type { PauseReason } from "../protocol/PauseReason";
import { treacheryCardPopup } from "../popups";
import { useGame } from "../state/context";
import { FactionChip } from "./FactionChip";
import { MonsterIcon } from "./MonsterIcon";
import { PopoverText } from "./PopoverText";
import { SpiceComponent } from "./SpiceComponent";

const KARAMA = 23;
const TRIP_TO_GAMONT = 32;

// A treachery card in a sentence: its art and name, with the card's popover.
function CardChip({ id }: { id: number }) {
  const { skin } = useGame();
  return (
    <PopoverText popover={treacheryCardPopup(skin, id)} className="chip ref-card">
      <img src={skin.cardImage(id)} alt="" /><span className="name">{skin.card(id)}</span>
    </PopoverText>
  );
}

function NexusChip({ faction }: { faction: Faction }) {
  const { skin } = useGame();
  const name = `${skin.faction(faction)} Nexus`;
  return <span className="chip ref-card" title={name}><img src={skin.url(skin.get("NexusCardImage_URL", faction))} alt="" /><span className="name">{name}</span></span>;
}

const Karama = () => <CardChip id={KARAMA} />;
const Spice2 = () => <SpiceComponent amount="2" size={20} />;

interface Wording {
  /** The actions that mean the waited player can act on this reason. */
  actions: string[];
  /** After "Waiting in case <faction>". */
  others: ReactNode;
  /** After "You may still". */
  you: ReactNode;
  /** After "You may", when the player can't act on it now. */
  cant: ReactNode;
}

const wording: Record<Exclude<PauseReason, "lookAtNextCard">, () => Wording> = {
  chooseFaction: () => ({ actions: ["FactionSelected"], others: "chooses a faction", you: "choose a faction", cant: "choose a faction" }),
  tradeFactions: () => ({ actions: ["FactionTradeOffered"], others: "trades factions", you: "trade factions", cant: "trade factions" }),
  enterDiscovery: () => ({ actions: ["DiscoveryEntered"], others: "enters a Discovery", you: "enter a Discovery", cant: "enter a Discovery" }),
  useTestingStation: () => ({ actions: ["TestingStationUsed"], others: "uses the Testing Station", you: "use the Testing Station", cant: "use the Testing Station" }),
  karamaSandworm: () => ({
    actions: ["KarmaMonster"],
    others: <>uses <Karama /> to make <MonsterIcon /> appear</>,
    you: <>use <Karama /> to make <MonsterIcon /> appear</>,
    cant: <>make <MonsterIcon /> appear with <Karama /></>,
  }),
  replaceCardWon: () => ({
    actions: ["ReplacedCardWon"],
    others: "replaces the card they won",
    you: "replace the card you won",
    cant: <>replace the card you won using <NexusChip faction="Ixian" /></>,
  }),
  setRevivalLimits: () => ({ actions: ["SetIncreasedRevivalLimits"], others: "sets revival limits", you: "set revival limits", cant: "set revival limits" }),
  emperorDiscard: () => ({
    actions: ["EmperorDiscarded"],
    others: <>discards cards for <Spice2 /> each</>,
    you: <>discard cards for <Spice2 /> each</>,
    cant: <>discard cards for <Spice2 /> each</>,
  }),
  karamaStopRevival: () => ({
    actions: ["KarmaRevivalPrevention"],
    others: <>uses <Karama /> to stop a revival</>,
    you: <>use <Karama /> to stop a revival</>,
    cant: <>stop a revival with <Karama /></>,
  }),
  revive: () => ({ actions: ["Revival", "RequestTleilaxuRevival", "AcceptOrCancelTleilaxuRevival"], others: "revives", you: "revive", cant: "revive" }),
  placeAmbassador: () => ({ actions: ["AmbassadorPlaced"], others: "places an ambassador", you: "place an ambassador", cant: "place an ambassador" }),
  revealNoField: () => ({ actions: ["RicheseRevealedNoField"], others: "reveals a No-Field", you: "reveal a No-Field", cant: "reveal a No-Field" }),
  removeTerror: () => ({ actions: ["TerrorPlanted"], others: "removes terror tokens", you: "remove terror tokens", cant: "remove terror tokens" }),
  plantTerror: () => ({ actions: ["TerrorPlanted"], others: "plants a terror token", you: "plant a terror token", cant: "plant a terror token" }),
  inflation: () => ({ actions: ["ChoamEconomics"], others: "plays their Inflation token", you: "play your Inflation token", cant: "play your Inflation token" }),
  tripToGamont: () => ({
    actions: ["ChoamRemoveForce"],
    others: <>plays <CardChip id={TRIP_TO_GAMONT} /></>,
    you: <>play <CardChip id={TRIP_TO_GAMONT} /></>,
    cant: <>remove a force with <CardChip id={TRIP_TO_GAMONT} /></>,
  }),
  choamDiscard: () => ({ actions: ["ChoamDiscarded"], others: "discards a card", you: "discard a card", cant: "discard a card" }),
  tradeCard: () => ({ actions: ["CardTraded"], others: "trades a card with their ally", you: "trade a card with your ally", cant: "trade a card with your ally" }),
};

const known = (reasons: PauseReason[]) => reasons.filter((r): r is Exclude<PauseReason, "lookAtNextCard"> => r !== "lookAtNextCard").map((r) => wording[r]());

function joinOr(parts: ReactNode[]): ReactNode {
  return parts.map((p, i) => <span key={i}>{i > 0 && " or "}{p}</span>);
}

/** The line other players read for a waited player, or null when it has no worded reason. */
export function WaitingLine({ faction, reasons }: { faction: Faction; reasons: PauseReason[] }) {
  const ws = known(reasons);
  if (ws.length === 0) return null;
  if (reasons.length === 1 && reasons[0] === "chooseFaction") return <>Waiting for <FactionChip faction={faction} /> to choose a faction</>;
  return <>Waiting in case <FactionChip faction={faction} /> {joinOr(ws.map((w) => w.others))}</>;
}

/**
 * The waited player's own lines: what they may still do, or, when their hand or spice rules all
 * of it out, that the game waits for them anyway and what it would take. Null when no reason is
 * worded.
 */
export function OwnLines({ view, reasons, style }: { view: GameView; reasons: PauseReason[]; style?: React.CSSProperties }) {
  const ws = known(reasons);
  if (ws.length === 0) return null;
  const available = new Set(view.actions.map((a) => a.eventType));
  const can = ws.filter((w) => w.actions.some((a) => available.has(a)));
  if (can.length > 0) {
    return <div className="text-dark fw-bold text-center" style={style}>You may still {joinOr(can.map((w) => w.you))}</div>;
  }
  return (
    <>
      <div className="text-dark fw-bold text-center" style={style}>Waiting for you to continue</div>
      <div className="text-dark text-center mt-1">You may {joinOr(ws.map((w) => w.cant))}.</div>
    </>
  );
}
