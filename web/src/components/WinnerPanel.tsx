// OtherComponents/WinnerPanel.razor: the winners, and the button that opens the story of the
// game unless the rules disable it. ActionPanel draws it after GameEnded (view.ending).
import type { EndingView } from "../generated/EndingView";
import { useGame } from "../state/context";
import { StoryComponent } from "./StoryComponent";

export function WinnerPanel({ ending }: { ending: EndingView }) {
  const { skin } = useGame();
  return (
    <div className="card p-1 mb-2" style={{ animation: "fadeIn 3s" }}>
      <div className="card-header">Victory!</div>
      <div className="card-body">
        {ending.winners.map((w, i) => (
          <h1 key={i} className="h1"><img width="72" src={skin.factionImage(w.faction)} /> {w.name}</h1>
        ))}
        {ending.story && (
          <>
            <button className="btn btn-primary mt-1" data-bs-toggle="modal" data-bs-target="#storyModal">Read the story of this game...</button>
            <StoryComponent id="storyModal" story={ending.story} />
          </>
        )}
      </div>
    </div>
  );
}
