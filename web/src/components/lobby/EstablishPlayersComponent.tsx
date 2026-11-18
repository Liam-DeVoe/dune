// GameEventComponents/EstablishPlayersComponent.razor: the host's game settings while players
// join, sent to the server every 3.6 s (updateSettings) so joiners see them, and the
// EstablishPlayers event that starts the game.
//
// Bots: seats that no user takes become bots when the game starts (version 170 and later), as on
// the C# server. "AI Players" lists the rules of RuleGroup.Bots ("Bots may not initiate
// alliances"); the rules that add bots are in no group, so the C# form never offered them either.
// A refused start is written to the console, as GameComponent.Request does.
import { Fragment, useEffect, useRef, useState } from "react";
import type { Faction, Rule } from "../../generated/enums";
import type { Event } from "../../generated/Event";
import type { Part } from "../../generated/Part";
import { useLobby } from "../../lobby/context";
import type { GameSettings } from "../../lobby/types";
import { Checkbox } from "../Checkbox";
import { ButtonRow } from "../ButtonRow";
import { JoinedPlayersPanel } from "./JoinedPlayersPanel";
import { ChecklistItem, SelectFactions, SelectNumber } from "./widgets";
import { Select } from "../Select";

const SETTINGS_KEY = "treachery.online;setting.establishPlayersSettings";

/** `EstablishPlayersComponent.EstablishPlayersSettings` as the C# client stores it: the settings
 * in the savegame format, with PascalCase members and enums as numbers. */
interface Stored {
  Settings: StoredSettings | null;
  AdvancedOptions: boolean;
  Ruleset: number;
}

interface StoredSettings {
  NumberOfPlayers: number;
  MaximumTurns: number;
  InitialRules: number[];
  AllowedFactionsInPlay: number[];
  AutoOpenEmptySeats: boolean;
  AsyncPlay: boolean;
  AsyncPlayMessageIntervalMinutes: number;
}

export function EstablishPlayersComponent() {
  const [client, state] = useLobby();
  const { lobby } = client;
  const g = state.game!;
  const gameId = g.gameId;
  const view = g.view;

  const rulesetRules = (name: string) => (lobby.data.rulesetRules[name] ?? []) as Rule[];
  const toStored = (s: GameSettings): StoredSettings => ({
    NumberOfPlayers: s.numberOfPlayers, MaximumTurns: s.maximumTurns,
    InitialRules: s.initialRules.map((r) => lobby.value("Rule", r)),
    AllowedFactionsInPlay: s.allowedFactionsInPlay.map((f) => lobby.value("Faction", f)),
    AutoOpenEmptySeats: s.autoOpenEmptySeats, AsyncPlay: s.asyncPlay, AsyncPlayMessageIntervalMinutes: s.asyncPlayMessageIntervalMinutes,
  });
  const fromStored = (s: StoredSettings): GameSettings => ({
    numberOfPlayers: s.NumberOfPlayers, maximumTurns: s.MaximumTurns,
    initialRules: s.InitialRules.map((r) => lobby.name<Rule>("Rule", r)),
    allowedFactionsInPlay: s.AllowedFactionsInPlay.map((f) => lobby.name<Faction>("Faction", f)),
    autoOpenEmptySeats: s.AutoOpenEmptySeats, asyncPlay: s.AsyncPlay, asyncPlayMessageIntervalMinutes: s.AsyncPlayMessageIntervalMinutes,
  });
  const initial = (): { settings: GameSettings; ruleset: string; advanced: boolean } => {
    // The field initializer, then the settings stored by the last Start (LoadSettings).
    let settings: GameSettings = {
      numberOfPlayers: 6, maximumTurns: 10, initialRules: rulesetRules("BasicGame"),
      allowedFactionsInPlay: lobby.members("Faction").filter((m) => m.name !== "None").map((m) => m.name as Faction),
      autoOpenEmptySeats: true, asyncPlay: false, asyncPlayMessageIntervalMinutes: 0,
    };
    // A select with a value outside its choices takes the first choice.
    let ruleset = lobby.data.availableRulesets[0] as string;
    let advanced = false;
    try {
      const stored = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "") as Stored | null;
      if (stored) {
        if (stored.Settings) settings = { ...settings, ...fromStored({ ...toStored(settings), ...stored.Settings }) };
        if (stored.Ruleset !== 0) ruleset = lobby.name("Ruleset", stored.Ruleset);
        advanced = stored.AdvancedOptions;
      }
    } catch {
      // Ignore locally stored settings.
    }
    return { settings, ruleset, advanced };
  };
  const [start] = useState(initial);
  const [settings, setSettings] = useState<GameSettings>(start.settings);
  const [ruleset, setRulesetState] = useState(start.ruleset);
  const [advanced, setAdvanced] = useState(start.advanced);
  const [validation, setValidation] = useState<Part[] | null>(null);
  const seed = useRef(Math.floor(Math.random() * 2147483647));

  const change = (patch: Partial<GameSettings>) => setSettings((s) => ({ ...s, ...patch }));

  // RulesetChanged: keep the bot rules, take the ruleset's rules.
  const setRuleset = (r: string) => {
    if (r === ruleset) return;
    setRulesetState(r);
    const bots = settings.initialRules.filter((rule) => lobby.data.ruleGroup[rule] === "Bots");
    change({ initialRules: [...bots, ...rulesetRules(r)] });
  };

  // The form sets the settings and the seed. The fields it leaves out take their defaults on the
  // server.
  const draft = (): Event => ({ type: "EstablishPlayers", settings, seed: seed.current }) as Extract<Event, { type: "EstablishPlayers" }>;

  // UpdateSettings: every 3.6 s while the form is open.
  const latest = useRef(settings);
  latest.current = settings;
  useEffect(() => {
    const t = window.setInterval(() => { void client.requestUpdateSettings(gameId, latest.current); }, 3600);
    return () => window.clearInterval(t);
  }, [client, gameId]);

  // Validation: the server validates the draft (QueryDraft) after every change.
  const action = view?.actions.find((a) => a.eventType === "EstablishPlayers");
  useEffect(() => {
    let live = true;
    if (!action) return;
    g.source.query(draft()).then((r) => { if (live) setValidation(r.validation ?? null); }).catch(() => {});
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, view?.eventNumber, g.participation]);

  const confirm = async () => {
    const stored: Stored = { Settings: toStored(settings), Ruleset: lobby.value("Ruleset", ruleset), AdvancedOptions: advanced };
    try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(stored)); } catch { /* Don't store settings. */ }
    const error = await client.requestGameEvent(draft());
    if (error != null) console.log(error);
  };
  const cancel = async () => { await client.requestCloseGame(gameId); };

  const toggleRule = (rule: Rule, on: boolean) => {
    change({ initialRules: on ? [...settings.initialRules, rule] : settings.initialRules.filter((r) => r !== rule) });
  };
  const sortedRules = (group: string) =>
    // OrderBy(r => Name(r)): ordinal in the browser's .NET (uppercase before lowercase).
    [...(lobby.data.rulesInGroup[group] ?? [])].sort((a, b) => {
      const la = lobby.labelOf("Rule", a);
      const lb = lobby.labelOf("Rule", b);
      return la < lb ? -1 : la > lb ? 1 : 0;
    });


  return (
    <div className="card card--urgent mb-2" style={{ animation: "fadeIn 1s" }}>
      <div className="card-body">
        <SelectNumber value={settings.maximumTurns} onChange={(v) => change({ maximumTurns: v })} min={1} max={lobby.data.maximumTurns} label="Maximum number of turns:" />
        <SelectNumber value={settings.numberOfPlayers} onChange={(v) => change({ numberOfPlayers: v })} min={2} max={lobby.data.maximumPlayers} label="Number of players:" />
        <SelectFactions values={lobby.data.availableFactions as Faction[]} selected={settings.allowedFactionsInPlay} imageWidth={40} label="Factions in play:"
                        onChange={(fs) => change({ allowedFactionsInPlay: fs })} />
        <Select required label="Ruleset:" value={ruleset} onChange={setRuleset}
                         options={lobby.data.availableRulesets.map((r) => ({ value: r as string, label: lobby.labelOf("Ruleset", r) }))} />
        <JoinedPlayersPanel />
        <ButtonRow pass={() => void cancel()} passText="Cancel game" confirm={() => void confirm()} confirmText="Start" confirmError={validation} />
        <Checkbox value={settings.autoOpenEmptySeats} onChange={(v) => change({ autoOpenEmptySeats: v })}>Auto open all bot seats to other players at start</Checkbox>
        <Checkbox value={settings.asyncPlay} onChange={(v) => change({ asyncPlay: v })}>Async play (send e-mail updates)</Checkbox>
        {settings.asyncPlay && (
          <SelectNumber value={settings.asyncPlayMessageIntervalMinutes} onChange={(v) => change({ asyncPlayMessageIntervalMinutes: v })} min={10} max={120} stepDown={10} stepUp={10} label="Minimum minutes between e-mails:" />
        )}
        <Checkbox value={advanced} onChange={setAdvanced}>Change advanced options</Checkbox>
        {advanced && (
          <>
            {/* AI Players: Game.GetRulesInGroup(RuleGroup.Bots, ...). */}
            <label className="mt-1 mb-0">AI Players</label>
            {(lobby.data.rulesInGroup["Bots"] ?? []).map((r) => {
              const v = r as Rule;
              return <ChecklistItem key={r} label={lobby.labelOf("Rule", r)} checked={settings.initialRules.includes(v)} onChange={(on) => toggleRule(v, on)} />;
            })}
          </>
        )}
        {advanced && lobby.data.availableRuleGroups.filter((group) => (lobby.data.rulesInGroup[group] ?? []).length > 0).map((group) => (
          <Fragment key={group}>
            <label className="mt-1 mb-0">{lobby.labelOf("RuleGroup", group)}</label>
            {sortedRules(group).map((r) => {
              const v = r as Rule;
              return <ChecklistItem key={r} label={lobby.labelOf("Rule", r)} checked={settings.initialRules.includes(v)} onChange={(on) => toggleRule(v, on)} />;
            })}
          </Fragment>
        ))}
      </div>
    </div>
  );
}
