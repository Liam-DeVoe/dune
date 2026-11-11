// OtherComponents/TurnBanner.razor: "Your turn" and how long the game has waited, across the top
// of the screen while the viewer is awaited. The game screen leaves room for it through
// --turn-banner-h (GameScreen.cs, turnBannerStyle below).
import type { GameView } from "../generated/GameView";
import { useGame } from "../state/context";
import { since, useClock } from "./turnClock";

// GameScreen.ShowsTurnBanner: a seated player the status waits for.
export function showsTurnBanner(view: GameView): boolean {
  return !view.viewer.isObserver && view.me !== null && view.status.waitingForMe;
}

// GameScreen.Style: the height the banner's spacer adds above the game screen.
export function turnBannerStyle(view: GameView): React.CSSProperties {
  return { ["--turn-banner-h" as string]: `${showsTurnBanner(view) ? 40 : 0}px` };
}

export function TurnBanner() {
  const { view } = useGame();
  if (!showsTurnBanner(view)) return null;
  return (
    <>
      <div className="turn-banner-space"></div>
      <div className="turn-banner" role="status">
        <span className="label">Your turn</span>{" "}
        <BannerClock view={view} />
      </div>
    </>
  );
}

// Since the event the game is waiting on, as the prompt's clock counts it, or else since the
// last thing that happened.
function BannerClock({ view }: { view: GameView }) {
  const clock = useClock();
  const from = view.status.timedEventTime ?? view.timing.lastActivity;
  return <span className="clock">{from ? since(clock, from) : ""}</span>;
}
