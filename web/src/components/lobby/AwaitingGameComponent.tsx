// What a non-host sees while the host sets up the
// game: the host's settings (sent every few seconds by UpdateSettings) and who has joined.
import type { RuleGroup } from "../../generated/enums";
import { useLobby } from "../../lobby/context";
import { JoinedPlayersPanel } from "./JoinedPlayersPanel";
import { FactionImage } from "./widgets";

const GROUPS: RuleGroup[] = ["CoreAdvanced", "CoreBasicExceptions", "CoreAdvancedExceptions", "ExpansionIxAndBtBasic", "ExpansionIxAndBtAdvanced", "House"];

export function AwaitingGameComponent() {
  const [client, state] = useLobby();
  const g = state.game;
  if (!g) return null;
  const { lobby } = client;
  const settings = g.settings;
  const rules = new Set<string>(settings.initialRules);
  return (
    <div className="card awaiting-game">
      <div className="card__header">Welcome to {g.gameName}</div>
      <div className="card__body">
        <div className="awaiting-game__setting"><strong>Maximum number of turns:</strong> {settings.maximumTurns}</div>
        <div className="awaiting-game__setting"><strong>Maximum number of players:</strong> {settings.numberOfPlayers}</div>
        <div className="awaiting-game__setting">
          {settings.allowedFactionsInPlay.map((f, i) => <FactionImage key={i} faction={f} />)}
        </div>
        <JoinedPlayersPanel />
        {GROUPS.map((group) => (
          <RuleList key={group} title={lobby.labelOf("RuleGroup", group)} rules={lobby.data.rulesInGroup[group] ?? []} applicable={(r) => rules.has(r)} />
        ))}
      </div>
    </div>
  );
}

function RuleList({ title, rules, applicable }: { title: string; rules: string[]; applicable: (rule: string) => boolean }) {
  const [client] = useLobby();
  return (
    <>
      <label className="awaiting-game__rules-title">{title}</label>
      <ul className="awaiting-game__rules">
        {rules.map((r) => (
          <li key={r} className="awaiting-game__rule" style={{ color: applicable(r) ? "var(--parchment)" : "var(--text-3)" }}>{client.lobby.labelOf("Rule", r)}</li>
        ))}
      </ul>
    </>
  );
}
