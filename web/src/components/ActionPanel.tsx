// OtherComponents/ActionPanel.razor: one form per applicable action, chosen by event type.
// Each form keeps its inputs as a draft event. When an input changes it asks the server for the
// draft's options and validation (a draft query), as the C# component re-ran Validate() and the
// Valid* helpers on every render.
import type { ActionView } from "../generated/ActionView";
import type { GameView } from "../generated/GameView";
import { useGame } from "../state/context";
import { otherForm, shownInActionPanel } from "./other/OtherForms";
import { shipMoveForm } from "./shipmove";
import { battleForm } from "./battle/BattleForms";
import { revivalForm } from "./revival/RevivalForms";
import { BiddingPanel, useAutoPassReset } from "./BiddingPanel";
import { WinnerPanel } from "./WinnerPanel";
import { PerformSetupForm } from "./lobby/PerformSetupForm";

// ActionPanel.Shows without a filter: everything except the treachery cards played from the faction
// panel (CardPlayPopover) and the trade panels opened from the menu bar (TradePopover).
function shows(view: GameView, a: ActionView): boolean {
  const playedFromHand = view.me?.hand.some((c) => c.playableWith.includes(a.eventType)) ?? false;
  return !playedFromHand && shownInActionPanel(a);
}

// `filter` (ActionPanel.Filter) draws only the actions it accepts, opened from a popover
// (CardPlayPopover); the winner, bidding and PlayerReplaced panels are then left out.
export function ActionPanel({ filter, inPopover = false }: { filter?: (a: ActionView) => boolean; inPopover?: boolean } = {}) {
  const { view } = useGame();
  useAutoPassReset();
  if (filter) {
    return <>{view.actions.filter(filter).map((a) => battleForm(a, inPopover) ?? revivalForm(a, inPopover) ?? shipMoveForm(a) ?? otherForm(a, inPopover))}</>;
  }
  return (
    <>
      {/* Phase.Bidding and Phase.BlackMarketBidding: the bidding panel, for bidders and others. */}
      {/* Phase.GameEnded: the winners and the story (package 9). */}
      {view.ending && <WinnerPanel ending={view.ending} />}
      {view.bidding?.panel && <BiddingPanel panel={view.bidding.panel} />}
      {view.actions.filter((a) => shows(view, a) && a.eventType !== "PlayerReplaced").map((a) => {
        switch (a.options?.form) {
          // Drawn by the bidding panel above.
          case "Bid": case "BlackMarketBid": return null;
          // Custom setup (package 8, components/lobby).
          case "PerformSetup": return <PerformSetupForm key={a.eventType} action={a} />;
          // The other events (package 7e, components/other); null for forms not built yet.
          // Ship and move (package 7c, components/shipmove), then the other events.
          // Battle (package 7b, components/battle) first, then revival (7d, components/revival).
          default: return battleForm(a) ?? revivalForm(a) ?? shipMoveForm(a) ?? otherForm(a);
        }
      })}
      {/* PlayerReplaced comes after the loop, until the game has ended. */}
      {view.mainPhase !== "Ended" && view.actions.filter((a) => a.eventType === "PlayerReplaced").map((a) => otherForm(a))}
    </>
  );
}
