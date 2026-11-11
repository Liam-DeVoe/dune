// OtherComponents/CardPlayPopover.razor: the panel for playing a treachery card, opened by
// clicking the card in your faction panel (PlayerPanel). It floats to the left of the side
// column, over the board, at the height the card was clicked, and shows the forms of every action
// that plays this card (the card's `playableWith`). It closes on a click outside, and draws
// nothing once the card can no longer be played. Styles: .card-play-pop in _menu.scss.
import { useGame } from "../state/context";
import { ActionPanel } from "./ActionPanel";
import { useMenu } from "./menuState";

export function CardPlayPopover() {
  const { view } = useGame();
  const menu = useMenu();
  const card = menu.cardPlay;
  const plays = card === null || view.viewer.isObserver ? [] : view.me?.hand.find((c) => c.id === card)?.playableWith ?? [];
  if (plays.length === 0) return null;
  return (
    <>
      <div className="menu-scrim" onClick={() => menu.closeCardPlay()} />
      <div className="card-play-pop" style={{ top: `${Math.max(8, menu.cardPlayY - 40)}px` }}>
        <ActionPanel filter={(a) => plays.includes(a.eventType)} inPopover />
      </div>
    </>
  );
}
