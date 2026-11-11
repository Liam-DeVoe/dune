// OtherComponents/ChatComponent.razor: the chat panel of the main page (Main.razor shows it in a
// HoverPanel beside the lobby and the game). It needs the connection's chat messages, the user's
// id and the names of the users the server has seen, which the page that owns the connection
// (the lobby client) has; so it takes them as props instead of reading the game view.
//
// Bodies are written as ChatMessage.GetBodyIncludingPlayerInfo writes them (GlobalChatMessage,
// GameChatMessage), with factions as faction chips.
import { useState } from "react";
import type { Faction } from "../generated/enums";
import type { Part } from "../generated/Part";
import { Checkbox } from "./Checkbox";
import { RichText } from "./RichText";

/** A GlobalChatMessage (`global`) or GameChatMessage, as the hub relays them. */
export interface ChatEntry {
  global: boolean;
  sourceUserId: number;
  /** GameChatMessage.TargetUserId: -1 for everyone in the game. */
  targetUserId: number;
  body: string;
}

export interface ChatProps {
  messages: ChatEntry[];
  userId: number;
  /** The name of a user the server has seen (RecentlySeenUsers), or null. */
  userName: (userId: number) => string | null;
  /** The faction a user plays in the current game (Game.GetPlayerByUserId), or null. */
  factionOf: (userId: number) => Faction | null;
  inGame: boolean;
  isObserver: boolean;
  /** The other players of the game, as chat targets: their names and user ids. */
  players: { name: string; userId: number }[];
  muteGlobalChat: boolean;
  setMuteGlobalChat: (mute: boolean) => void;
  sendGlobal: (body: string) => void;
  sendGame: (targetUserId: number, body: string) => void;
  backgroundColor?: string;
}

const LABEL_GLOBAL = "all on server";
const LABEL_ALL_IN_GAME = "all in game";

const text = (v: string): Part => ({ t: "text", v });
const faction = (f: Faction): Part => ({ t: "faction", v: f, label: f });

// ChatMessage.GetBodyIncludingPlayerInfo.
function body(m: ChatEntry, p: ChatProps): Part[] {
  const name = (id: number) => p.userName(id) ?? "Offline player";
  if (m.global) {
    const contextIsGlobal = !p.inGame;
    const suffix = contextIsGlobal ? [] : [text(" ⇒ GLOBAL")];
    return m.sourceUserId === p.userId ? [text("You: "), text(m.body), ...suffix] : [text(name(m.sourceUserId)), text(": "), text(m.body), ...suffix];
  }
  if (m.sourceUserId === p.userId) {
    if (m.targetUserId < 0) return [text("You: "), text(m.body), text(" ⇒ ALL")];
    const f = p.factionOf(m.targetUserId);
    return [text("You: "), text(m.body), text(" ⇒ "), f ? faction(f) : text(name(m.targetUserId))];
  }
  const f = p.factionOf(m.sourceUserId);
  const who = f ? faction(f) : text(name(m.sourceUserId));
  return m.targetUserId < 0 ? [who, text(" (to ALL) "), text(m.body)] : [who, text(" "), text(m.body)];
}

export function ChatComponent(p: ChatProps) {
  const showGameChat = p.inGame && !p.isObserver;
  const targets = [...(p.muteGlobalChat ? [] : [LABEL_GLOBAL]), LABEL_ALL_IN_GAME, ...p.players.map((x) => x.name)];
  const [message, setMessage] = useState("");
  const [target, setTarget] = useState(showGameChat ? (p.players[0]?.name ?? LABEL_ALL_IN_GAME) : LABEL_GLOBAL);
  const selected = showGameChat ? target : LABEL_GLOBAL;

  const send = () => {
    if (message === "") return;
    if (selected === LABEL_GLOBAL) p.sendGlobal(message);
    else p.sendGame(selected === LABEL_ALL_IN_GAME ? -1 : p.players.find((x) => x.name === selected)?.userId ?? -1, message);
    setMessage("");
  };

  return (
    <div className="card border-none" style={{ backgroundColor: p.backgroundColor ?? "transparent", border: "none" }}>
      <div className="card-header">Chat</div>
      <div className="card-body">
        <div className="d-flex flex-row-reverse">
          <input name="message" className="form-control form-control-sm" type="text" maxLength={512} placeholder={`Chat to ${selected}...`}
                 value={message} onChange={(e) => setMessage(e.target.value)}
                 onKeyUp={(e) => { if (e.code === "Enter" || e.code === "NumpadEnter") send(); }} />
        </div>
        <div className="d-flex flex-row-reverse mt-1">
          <button className="btn btn-primary btn-sm" onClick={send}>Send</button>
          {showGameChat && (
            <select className="form-select form-select-sm me-1 flex-grow-1" value={selected} onChange={(e) => setTarget(e.target.value)}>
              {targets.map((t) => <option key={t} value={t}>{t}</option>)}
            </select>
          )}
        </div>
        {p.messages.filter((m) => !p.muteGlobalChat || !m.global).map((m, i) => (
          <div key={i} className="text-break p-1 mt-1 small bg-ink-3" style={{ userSelect: "text" }}>
            <RichText parts={body(m, p)} size={20} />
          </div>
        ))}
        {showGameChat && (
          <div className="mt-1">
            <Checkbox value={p.muteGlobalChat} onChange={p.setMuteGlobalChat}>Hide global chat messages</Checkbox>
          </div>
        )}
      </div>
    </div>
  );
}
