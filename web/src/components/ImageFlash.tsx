// OtherComponents/ImageFlashComponent.razor: the cards, leaders or factions the latest event
// revealed (status.flash) flash over the board for a few seconds, centred in the window.
import { useEffect, useState, type CSSProperties } from "react";
import type { GameView } from "../generated/GameView";
import type { Part } from "../generated/Part";
import { initialFlash, nextFlash } from "../flashTrigger";
import type { Skin } from "../skin";
import { useGame } from "../state/context";
import { RichText } from "./RichText";

const SPACING_X = Math.fround(1.1);

// Skin.GetImageUrl(object) for what a flash shows.
function imageUrl(skin: Skin, p: Part): string {
  switch (p.t) {
    case "faction": return skin.url(skin.get("FactionImage_URL", p.v));
    case "treacheryCard": return skin.cardImage(p.id);
    case "leader": return skin.personImage(p.id);
    case "messiah": return skin.url(skin.str("Messiah_URL"));
    case "resourceCard": return skin.url(skin.get("ResourceCardImage_URL", p.id));
    case "enum": {
      const dict: Record<string, string> = {
        LeaderSkill: "LeaderSkillCardImage_URL", TechToken: "TechTokenImage_URL", Nexus: "NexusCardImage_URL", Ambassador: "AmbassadorImage_URL",
        StrongholdAdvantage: "StrongholdCardImage_URL", FactionForce: "FactionForceImage_URL", FactionSpecialForce: "FactionSpecialForceImage_URL",
      };
      return dict[p.ty] ? skin.url(skin.get(dict[p.ty], p.v)) : "";
    }
    default: return "";
  }
}

function name(p: Part): string {
  return "label" in p ? p.label : "";
}

// Support.TextBorder(2, "var(--ink-0)").
const border: CSSProperties = { WebkitTextStroke: "0.6px var(--ink-0)", textShadow: "0 0 4px var(--ink-0)" };

// `view` is the live game's view, not an undo preview's: leaving a preview must not flash again.
export function ImageFlash({ view }: { view: GameView }) {
  const { skin } = useGame();
  // Deviation 6 (protocol.md): a flash after every new event that brings one, not only on the
  // first render as in C# (flashTrigger.ts).
  const [state, setState] = useState(() => initialFlash(view.eventNumber, view.status.flash));
  const next = nextFlash(state, view.eventNumber, view.status.flash);
  if (next !== state) setState(next);
  // The window size when the flash starts (ImageFlashComponent.ApplyDimensions).
  const [size, setSize] = useState({ key: state.key, w: window.innerWidth, h: window.innerHeight });
  if (size.key !== next.key) setSize({ key: next.key, w: window.innerWidth, h: window.innerHeight });
  const [hidden, setHidden] = useState(0);
  useEffect(() => {
    if (next.time <= 0) return;
    const t = window.setTimeout(() => setHidden(next.key), next.time);
    return () => window.clearTimeout(t);
  }, [next.key, next.time]);
  if (next.time <= 0 || hidden === next.key) return null;

  const { w, h } = size;
  const { items: flash, time } = next;
  const n = flash.length;
  let cardWidth: number;
  let windowY: number;
  if (h > w) {
    cardWidth = Math.trunc(Math.fround(0.3) * w);
    windowY = Math.trunc(0.5 * (h - (n <= 3 ? 1 : 2) * Math.fround(1.41) * cardWidth));
  } else {
    cardWidth = Math.trunc(Math.fround(0.15) * w);
    windowY = Math.trunc(0.5 * (h - (n <= 4 ? 1 : 2) * Math.fround(1.41) * cardWidth));
  }
  const itemWidth = Math.trunc(SPACING_X * cardWidth);
  const flashWidth = n * (itemWidth + 20);
  const windowX = Math.trunc(0.5 * (w - flashWidth));
  return (
    <div key={next.key} data-flash={next.onOpen ? "opened" : "event"} style={{ position: "absolute", left: `${windowX}px`, top: `${windowY}px`, width: `${flashWidth}px`, overflow: "hidden", animation: `fadeInFadeOut ${time}ms`, zIndex: 1000 }}
      onClick={() => setHidden(next.key)}>
      {flash.map((img, i) => (
        <div key={i} style={{ float: "left", position: "relative", top: "0px", display: "inline-block", verticalAlign: "top", width: `${itemWidth}px` }}>
          <img src={imageUrl(skin, img.toShow)} width={cardWidth} style={{ filter: "drop-shadow(2px 2px 2px black)" }} alt={name(img.toShow)} />
          {img.message.length > 0 && (
            <div style={{ width: `${cardWidth}px`, font: "normal normal normal 24px var(--font-display)", color: "var(--parchment)", textAlign: "center" }}>
              <RichText parts={img.message} size={24} textBorder={border} bordered={img.bordered} />
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
