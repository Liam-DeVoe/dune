// The lobby client and its state, for the lobby and setup screens.
import { createContext, useContext, useSyncExternalStore } from "react";
import type { LobbyClient, LobbyState } from "./client";

export const LobbyContext = createContext<LobbyClient | null>(null);

export function useLobbyClient(): LobbyClient {
  const c = useContext(LobbyContext);
  if (!c) throw new Error("useLobbyClient outside the lobby");
  return c;
}

/** The client and its current state; the component redraws whenever the state changes. */
export function useLobby(): [LobbyClient, LobbyState] {
  const client = useLobbyClient();
  const state = useSyncExternalStore(client.subscribe, client.getState);
  return [client, state];
}
