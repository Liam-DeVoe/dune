// OtherComponents/WinnerPanel.razor: the winners, and the button that opens the story of the
// game unless the rules disable it. ActionPanel draws it after GameEnded (view.ending).
import type { EndingView } from "../generated/EndingView";
import { useGame } from "../state/context";
import { StoryComponent } from "./StoryComponent";

export function WinnerPanel({ ending }: { ending: EndingView }) {
  const { skin } = useGame();
  return (
    <div className="card winner-panel" style={{ animation: "fadeIn 3s" }}>
      <div className="card__header">Victory!</div>
      <div className="card__body">
        {ending.winners.map((w, i) => (
          <h1 key={i}><img width="72" src={skin.factionImage(w.faction)} /> {w.name}</h1>
        ))}
        {ending.story && (
          <>
            <button className="button button--primary winner-panel__story" data-bs-toggle="modal" data-bs-target="#storyModal">Read the story of this game...</button>
            <StoryComponent id="storyModal" story={ending.story} />
          </>
        )}
      </div>
    </div>
  );
}
