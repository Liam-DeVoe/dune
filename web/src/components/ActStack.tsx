// The act stack (GameScreen): the status card and the action panels under it, joined into one
// parchment card. A card drawn inside it marks itself card--act (the parchment surface) and
// card--joined (no gap or rounded corners between it and its neighbours); see
// app/_prompt-act.scss. The same panels opened from a popover or a menu panel stay ink.
import { createContext, useContext, type ReactNode } from "react";

const InActStack = createContext(false);

export function ActStack({ children }: { children: ReactNode }) {
  return <InActStack.Provider value={true}><div className="act-stack">{children}</div></InActStack.Provider>;
}

// Whether the component is drawn inside the act stack.
export function useInActStack(): boolean {
  return useContext(InActStack);
}

// The modifiers of an action panel's card: parchment and joined in the act stack, none elsewhere.
export function useActCard(): string {
  return useInActStack() ? "card--act card--joined" : "";
}
