// Renders log lines with RichText on the server side and returns the text each shows, for
// check-rich-text.ts. Loaded through Vite's SSR loader, so it sees the app's modules as the
// browser does.
import { renderToStaticMarkup } from "react-dom/server";
import { RichText } from "../src/components/RichText";
import type { GameView } from "../src/generated/GameView";
import type { Part } from "../src/generated/Part";
import { Skin, type SkinData, type StaticData } from "../src/skin";
import { Game, type GameContext } from "../src/state/context";

// textContent of rendered markup. React escapes < and > inside attribute values, so tags can be
// removed by pattern.
function textContent(html: string): string {
  return html
    .replace(/<[^>]*>/g, "")
    .replace(/&#x([0-9a-f]+);/gi, (_m, h: string) => String.fromCodePoint(parseInt(h, 16)))
    .replace(/&#(\d+);/g, (_m, d: string) => String.fromCodePoint(parseInt(d, 10)))
    .replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&nbsp;/g, " ").replace(/&amp;/g, "&");
}

export function render(lines: Part[][], skinData: SkinData, statics: StaticData): { text: string; html: string }[] {
  const skin = new Skin(skinData, statics);
  // An observer's view with an empty board: parts only read the skin, the viewer and the
  // board's attached locations.
  const view = { viewer: { faction: null, isHost: false, isObserver: true }, board: { forces: [], attached: [] } } as unknown as GameView;
  const ctx: GameContext = { view, skin, source: null as unknown as GameContext["source"] };
  return lines.map((parts) => {
    const html = renderToStaticMarkup(<Game.Provider value={ctx}><RichText parts={parts} size={20} logStyle /></Game.Provider>);
    return { text: textContent(html), html };
  });
}
