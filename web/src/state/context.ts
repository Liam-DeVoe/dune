import { createContext, useContext } from "react";
import type { GameView } from "../generated/GameView";
import type { Skin } from "../skin";
import type { ViewSource } from "./source";

export interface GameContext {
  view: GameView;
  skin: Skin;
  source: ViewSource;
}

export const Game = createContext<GameContext | null>(null);

export function useGame(): GameContext {
  const g = useContext(Game);
  if (!g) throw new Error("useGame outside a game screen");
  return g;
}
