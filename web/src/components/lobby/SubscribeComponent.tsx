// SubscribeComponent and UserStatusComponent: a dropdown button with
// the current choice, listing the others.
import { useState } from "react";
import { useLobby } from "../../lobby/context";
import type { ScheduledGameInfo, SubscriptionType, UserStatus } from "../../lobby/types";
import { DropdownButton } from "./widgets";

export function SubscribeComponent({ scheduledGame }: { scheduledGame: ScheduledGameInfo }) {
  const [client] = useLobby();
  const { lobby } = client;
  const [busy, setBusy] = useState(false);
  const current: SubscriptionType = scheduledGame.subscribers.find((s) => s.userId === client.userId)?.subscription ?? "DontParticipate";
  const btn = current === "DontParticipate" ? "btn-danger"
    : current === "MaybeAsPlayer" || current === "MaybeAsHost" ? "btn-warning"
    : current === "CertainAsPlayer" || current === "CertainAsHost" ? "btn-primary" : "btn-dark";
  const set = async (s: SubscriptionType) => {
    setBusy(true);
    await client.requestSubscribeGame(scheduledGame.scheduledGameId, s);
    setBusy(false);
  };
  const items = lobby.members("SubscriptionType").filter((m) => m.name !== "None" && m.name !== current)
    .map((m) => ({ label: m.label, onClick: () => void set(m.name as SubscriptionType) }));
  return <DropdownButton groupClass="btn-group align-self-end" buttonClass={btn} label={lobby.labelOf("SubscriptionType", current)} items={items} disabled={busy} />;
}

export function UserStatusComponent() {
  const [client] = useLobby();
  const { lobby } = client;
  const [busy, setBusy] = useState(false);
  // "None" until the lobby lists the user.
  const status = client.userStatus() ?? "None";
  const btn = status === "Online" ? "btn-success" : status === "Lfg" || status === "Lfm" ? "btn-primary" : status === "InGame" ? "btn-secondary" : "btn-dark";
  const set = async (s: UserStatus) => {
    setBusy(true);
    await client.requestSetUserStatus(s);
    setBusy(false);
  };
  const items = lobby.members("UserStatus").filter((m) => m.name !== "None" && m.name !== status)
    .map((m) => ({ label: m.label, onClick: () => void set(m.name as UserStatus) }));
  return <DropdownButton buttonClass={btn} label={lobby.labelOf("UserStatus", status)} items={items} disabled={busy} />;
}
