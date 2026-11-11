// OtherComponents/MusicComponent.razor: the volume sliders, drawn inside the menu. The component
// stays mounted while they are hidden, because it also plays the music: a soundtrack per main
// phase from the skin. IGameService.ChatEnabled is false in C#, so there is no chat volume slider.
import { useEffect, useRef, useState } from "react";
import { PhaseValue } from "../generated/enums";
import type { GameView } from "../generated/GameView";
import type { Skin } from "../skin";
import { changeSoundVolume, playSound, saveSoundSettings, soundSettings, stopSound, stopSounds } from "../sound";
import { useGame } from "../state/context";

// MusicComponent.DetermineSoundtrack.
function soundtrack(view: GameView, skin: Skin): string {
  const s = (key: string) => skin.str(key);
  switch (view.mainPhase) {
    case "Setup": return s("MusicSetup_URL");
    case "Blow": case "Charity": return s("MusicResourceBlow_URL");
    case "Bidding": return s("MusicBidding_URL");
    case "ShipmentAndMove": return s("MusicShipmentAndMove_URL");
    case "Battle": return PhaseValue[view.phase] >= PhaseValue.CallTraitorOrPass ? s("MusicBattleClimax_URL") : s("MusicBattle_URL");
    case "Collection": case "Contemplate": case "Ended": return s("MusicMentat_URL");
    default: return s("MusicGeneral_URL");
  }
}

export function MusicComponent({ visible }: { visible: boolean }) {
  const { view, skin } = useGame();
  const [music, setMusic] = useState(soundSettings.music);
  const [effect, setEffect] = useState(soundSettings.effect);
  const current = useRef({ soundtrack: "", volume: -1 });

  // UpdateMusicAndSound, after every render.
  useEffect(() => {
    const play = soundtrack(view, skin);
    const c = current.current;
    if (c.soundtrack !== play) {
      if (c.soundtrack !== "") stopSound(c.soundtrack);
      c.soundtrack = play;
      playSound(play, music, true);
    } else if (Math.abs(c.volume - music) > 0.001) {
      c.volume = music;
      if (music <= 4) {
        stopSounds();
        c.soundtrack = "";
      } else {
        changeSoundVolume(play, music);
      }
    }
  });

  // Leaving the game stops the music. (C# kept the last soundtrack playing on the main screen,
  // where the web client has no sound controls.)
  useEffect(() => () => stopSounds(), []);

  if (!visible) return null;
  return (
    <div className="volume">
      <label htmlFor="musicVolume">Music volume</label>
      <input id="musicVolume" className="form-range" type="range" min="0" max="100" step="1" value={music}
        onChange={(e) => { soundSettings.music = Number(e.currentTarget.value); setMusic(soundSettings.music); }} onPointerUp={saveSoundSettings} onKeyUp={saveSoundSettings} />

      <label htmlFor="effectVolume">Sound effects volume</label>
      <input id="effectVolume" className="form-range" type="range" min="0" max="100" step="1" value={effect}
        onChange={(e) => { soundSettings.effect = Number(e.currentTarget.value); setEffect(soundSettings.effect); }} onPointerUp={saveSoundSettings} onKeyUp={saveSoundSettings} />
    </div>
  );
}
