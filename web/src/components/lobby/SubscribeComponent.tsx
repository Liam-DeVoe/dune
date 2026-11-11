// OtherComponents/SubscribeComponent.razor and UserStatusComponent.razor: a dropdown button with
// the current choice, listing the others.
import { useState } from "react";
import { useLobby } from "../../lobby/context";
import { dictGet, type ScheduledGameInfo } from "../../lobby/types";
import { DropdownButton } from "./widgets";

export function SubscribeComponent({ scheduledGame }: { scheduledGame: ScheduledGameInfo }) {
  const [client] = useLobby();
  const { lobby } = client;
  const [busy, setBusy] = useState(false);
  const dontParticipate = lobby.value("SubscriptionType", "DontParticipate");
  const current = dictGet(scheduledGame.Subscribers, client.userId) ?? dontParticipate;
  const name = lobby.name("SubscriptionType", current);
  const btn = name === "DontParticipate" ? "btn-danger"
    : name === "MaybeAsPlayer" || name === "MaybeAsHost" ? "btn-warning"
    : name === "CertainAsPlayer" || name === "CertainAsHost" ? "btn-primary" : "btn-dark";
  const set = async (s: number) => {
    setBusy(true);
    await client.requestSubscribeGame(scheduledGame.ScheduledGameId, s);
    setBusy(false);
  };
  const items = lobby.members("SubscriptionType").filter((m) => m.name !== "None" && m.value !== current)
    .map((m) => ({ label: m.label, onClick: () => void set(m.value) }));
  return <DropdownButton groupClass="btn-group align-self-end" buttonClass={btn} label={lobby.label("SubscriptionType", current)} items={items} disabled={busy} />;
}

export function UserStatusComponent() {
  const [client] = useLobby();
  const { lobby } = client;
  const [busy, setBusy] = useState(false);
  const status = client.userStatus();
  const name = lobby.name("UserStatus", status);
  const btn = name === "Online" ? "btn-success" : name === "Lfg" || name === "Lfm" ? "btn-primary" : name === "InGame" ? "btn-secondary" : "btn-dark";
  const set = async (s: number) => {
    setBusy(true);
    await client.requestSetUserStatus(s);
    setBusy(false);
  };
  const items = lobby.members("UserStatus").filter((m) => m.name !== "None" && m.value !== status)
    .map((m) => ({ label: m.label, onClick: () => void set(m.value) }));
  return <DropdownButton buttonClass={btn} label={lobby.label("UserStatus", status)} items={items} disabled={busy} />;
}
