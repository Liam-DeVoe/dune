// OtherComponents/AwaitingGameComponent.razor: what a non-host sees while the host sets up the
// game: the host's settings (sent every few seconds by UpdateSettings) and who has joined.
import type { Faction, RuleGroup } from "../../generated/enums";
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
  const rules = new Set(settings.InitialRules);
  return (
    <div className="card mb-2">
      <div className="card-header">Welcome to {g.gameName}</div>
      <div className="card-body">
        <div className="mt-1 mb-0"><strong>Maximum number of turns:</strong> {settings.MaximumTurns}</div>
        <div className="mt-1 mb-0"><strong>Maximum number of players:</strong> {settings.NumberOfPlayers}</div>
        <div className="mt-1 mb-0">
          {settings.AllowedFactionsInPlay.map((f, i) => <FactionImage key={i} faction={lobby.name<Faction>("Faction", f)} />)}
        </div>
        <JoinedPlayersPanel />
        {GROUPS.map((group) => (
          <RuleList key={group} title={lobby.labelOf("RuleGroup", group)} rules={lobby.data.rulesInGroup[group] ?? []} applicable={(r) => rules.has(lobby.value("Rule", r))} />
        ))}
      </div>
    </div>
  );
}

function RuleList({ title, rules, applicable }: { title: string; rules: string[]; applicable: (rule: string) => boolean }) {
  const [client] = useLobby();
  return (
    <>
      <label className="mt-1 mb-0 small">{title}</label>
      <ul className="mt-0 mb-1">
        {rules.map((r) => (
          <li key={r} className="small" style={{ color: applicable(r) ? "var(--parchment)" : "var(--text-3)" }}>{client.lobby.labelOf("Rule", r)}</li>
        ))}
      </ul>
    </>
  );
}
