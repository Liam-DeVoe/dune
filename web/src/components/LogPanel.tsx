// OtherComponents/ReportComponent.razor: the game's log, one phase at a time. The arrows step
// through every phase of the game, and the dots jump within the turn on show. The panel follows
// each new phase as it starts, unless the player has stepped back to an earlier one. Then it
// stays put until they return to the latest phase.
//
// The view carries every report's header and the lines of the latest phase (log.latest). The
// lines of an earlier phase come from the source (RequestReport) and are cached per source; a
// cached report is asked again when a newer view arrives, and shown as cached meanwhile.
import { Fragment, type ReactNode, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { BattleFact } from "../generated/BattleFact";
import type { BattlePlans } from "../generated/BattlePlans";
import { type Faction, type MainPhase, MainPhaseValue } from "../generated/enums";
import type { LogLine } from "../generated/LogLine";
import type { ReportHeader } from "../generated/ReportHeader";
import type { ReportLines } from "../generated/ReportLines";
import { followBottom } from "../followLog";
import { useGame } from "../state/context";
import type { ViewSource } from "../state/source";
import { supportRound } from "../text";
import type { Board } from "./board/geometry";
import { BiddingLog } from "./log/BiddingLog";
import { BlowLog } from "./log/BlowLog";
import { CharityLog } from "./log/CharityLog";
import { CollectionLog } from "./log/CollectionLog";
import { forceLocations, ForceImage, HeroImage, Line, PartText, type PhaseLogProps, Plan, samePart } from "./log/common";
import { MentatLog } from "./log/MentatLog";
import { RevivalLog } from "./log/RevivalLog";
import { ShipMoveLog } from "./log/ShipMoveLog";
import { StormLog } from "./log/StormLog";
import { PlaceChip } from "./PlaceChip";
import { SimpleNumber } from "./SimpleNumber";
import { SpiceComponent } from "./SpiceComponent";
import { useBoardGeometry } from "./shipmove/common";

// The main phases of a turn, one dot each.
const turnPhases = (Object.entries(MainPhaseValue) as [MainPhase, number][])
  .filter(([, v]) => v >= MainPhaseValue.Storm && v <= MainPhaseValue.Contemplate)
  .sort((a, b) => a[1] - b[1])
  .map(([p]) => p);

// Every report of one phase of one turn. The battle phase has one report for the phase itself
// and one per battle.
interface Step { turn: number; about: MainPhase; reports: ReportHeader[] }

function steps(reports: ReportHeader[]): Step[] {
  const result: Step[] = [];
  for (const r of reports) {
    const last = result[result.length - 1];
    if (last && last.turn === r.turn && last.about === r.about) last.reports.push(r);
    else result.push({ turn: r.turn, about: r.about, reports: [r] });
  }
  return result;
}

// The lines of earlier reports, per source: what RequestReport answered, and for which view.
interface Cached { eventNumber: number; lines: LogLine[] }

class ReportCache {
  private readonly entries = new Map<number, Cached>();
  private readonly pending = new Set<string>();
  private readonly listeners = new Set<() => void>();
  private version = 0;

  constructor(private readonly source: ViewSource) {}

  get(index: number): Cached | undefined { return this.entries.get(index); }

  /** Asks for a report unless it is cached for this view or already asked for. */
  want(index: number, eventNumber: number): void {
    if (!this.source.report || this.entries.get(index)?.eventNumber === eventNumber) return;
    const key = `${index}@${eventNumber}`;
    if (this.pending.has(key)) return;
    this.pending.add(key);
    this.source.report(index).then(
      (r: ReportLines) => {
        this.pending.delete(key);
        const old = this.entries.get(index);
        if (old && old.eventNumber > eventNumber) return;
        this.entries.set(index, { eventNumber, lines: r.lines });
        this.version++;
        this.listeners.forEach((l) => l());
      },
      () => { this.pending.delete(key); },
    );
  }

  subscribe = (l: () => void): (() => void) => {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  };

  snapshot = (): number => this.version;
}

const caches = new WeakMap<ViewSource, ReportCache>();

function cacheOf(source: ViewSource): ReportCache {
  let c = caches.get(source);
  if (!c) caches.set(source, (c = new ReportCache(source)));
  return c;
}

export function LogPanel() {
  const { view, skin, source } = useGame();
  const board = useBoardGeometry();
  const cache = cacheOf(source);
  useSyncExternalStore(cache.subscribe, cache.snapshot);

  // The phase on show, or null to follow the latest one.
  const [selected, setSelected] = useState<{ turn: number; about: MainPhase } | null>(null);

  const all = useMemo(() => steps(view.log.reports), [view.log.reports]);
  const found = selected ? all.findIndex((s) => s.turn === selected.turn && s.about === selected.about) : -1;
  const index = found >= 0 ? found : all.length - 1;
  const step = all[index] as Step | undefined;
  const latestStep = index === all.length - 1;

  // Earlier phases' lines come from the source; the one before the shown phase is asked for
  // ahead, so stepping back is immediate.
  useEffect(() => {
    for (const i of [index, index - 1]) {
      if (i < 0 || i >= all.length - 1) continue;
      for (const r of all[i].reports) cache.want(r.index, view.eventNumber);
    }
  }, [cache, all, index, view.eventNumber]);

  if (!step) return null;

  const latestLines = new Map(view.log.latest.map((r) => [r.index, r.lines]));
  const linesOf = (r: ReportHeader): LogLine[] | undefined => (latestStep ? latestLines.get(r.index) : cache.get(r.index)?.lines);
  const loaded = step.reports.every((r) => linesOf(r) !== undefined);
  const lastReport = view.log.reports[view.log.reports.length - 1];

  const empty = step.reports.every((r) => !r.isBattle && (linesOf(r) ?? []).length === 0);
  // The latest phase may still be under way, so an empty log there just means nothing has happened yet.
  const over = index < all.length - 1 || view.mainPhase === "Ended";

  // Choosing the latest phase goes back to following.
  const select = (i: number) => {
    if (i < 0 || i >= all.length) return;
    setSelected(i === all.length - 1 ? null : { turn: all[i].turn, about: all[i].about });
  };

  const line = (l: LogLine) => <Line line={l} board={board} />;

  return (
    <div className="card report pb-1" style={{ animation: "fadeIn 1s" }}>
      <div className="card-header stepper">
        <button title="Previous phase" aria-label="Previous phase" disabled={index === 0} onClick={() => select(index - 1)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3 5 8l5 5" /></svg>
        </button>
        <div className="title">
          <div className="name">
            {step.turn !== view.turn && step.about !== "Setup" && <span className="turn">Turn {step.turn}</span>}
            <span className="phase">{skin.enumValue("MainPhase", step.about)}</span>
          </div>
          {MainPhaseValue[step.about] >= MainPhaseValue.Storm && MainPhaseValue[step.about] <= MainPhaseValue.Contemplate && (
            <div className="dots">
              {turnPhases.map((phase) => {
                const target = all.findIndex((s) => s.turn === step.turn && s.about === phase);
                const title = skin.enumValue("MainPhase", phase);
                if (target < 0) return <span key={phase} className="future" title={title}></span>;
                const cls = [target === all.length - 1 ? "now" : "", target === index ? "sel" : ""].filter((c) => c).join(" ");
                return <span key={phase} className={cls || undefined} title={title} onClick={() => select(target)}></span>;
              })}
            </div>
          )}
        </div>
        <button title="Next phase" aria-label="Next phase" disabled={index === all.length - 1} onClick={() => select(index + 1)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3l5 5-5 5" /></svg>
        </button>
      </div>

      {/* Follows the newest line while the player is at the bottom (followLog.ts). */}
      <div className="card-body p-1 overflow-auto" ref={followBottom} data-step={`${step.turn}-${step.about}`}>
        {!loaded ? null : empty ? (
          over && <div className="mt-1 ps-1 pe-1"><i>Nothing to report</i></div>
        ) : (
          step.reports.map((report) => {
            const lines = linesOf(report) ?? [];
            const latest = report.index === lastReport?.index;
            return report.isBattle
              ? <BattleReport key={report.index} report={report} lines={lines} line={line} board={board} />
              : <Fragment key={report.index}>{phaseLog(report.about, { lines, line, latest })}</Fragment>;
          })
        )}
      </div>
    </div>
  );
}

// Each main phase draws its own log design; phases without one are plain lines.
function phaseLog(about: MainPhase, props: PhaseLogProps): ReactNode {
  switch (about) {
    case "Storm": return <StormLog {...props} />;
    case "Blow": return <BlowLog {...props} />;
    case "Charity": return <CharityLog {...props} />;
    case "Bidding": return <BiddingLog {...props} />;
    case "Resurrection": return <RevivalLog {...props} />;
    case "ShipmentAndMove": return <ShipMoveLog {...props} />;
    case "Collection": return <CollectionLog {...props} />;
    case "Contemplate": return <MentatLog {...props} />;
    default: return <>{props.lines.map((l, i) => <Fragment key={i}>{props.line(l)}</Fragment>)}</>;
  }
}

function BattleReport({ report, lines, line, board }: { report: ReportHeader; lines: LogLine[]; line: (l: LogLine) => ReactNode; board: Board }) {
  const { skin } = useGame();
  const facts = outcomeFacts(report, lines);
  const territoryLocations = report.territory == null ? "" : (skin.territoryInfo(report.territory)?.locations ?? []).join(",");
  return (
    <div className={`battle ${report.battleDecided ? "" : "live"}`}>
      <div className="faceoff">
        <Side report={report} faction={report.aggressor} board={board} />
        <div className="middle" data-ref-locations={territoryLocations}>
          <div className="vs">vs</div>
          {report.territory != null && <div className="where"><PlaceChip place={{ territory: report.territory }} /></div>}
        </div>
        <Side report={report} faction={report.defender} board={board} />
      </div>
      {lines.map((l, i) => {
        if (facts.has(l)) return null;
        if (l.plans) return <Outcome key={i} report={report} plans={l.plans} facts={[...facts].map((m) => m.fact!)} />;
        return <Fragment key={i}>{line(l)}</Fragment>;
      })}
    </div>
  );
}

// One side of a battle heading: sigil over name. Once the battle is decided the winner's sigil
// gets a gold ring and the loser fades; with no winner both fade. Hovering it lights the
// faction's forces on the map, like a faction chip.
function Side({ report, faction, board }: { report: ReportHeader; faction: Faction; board: Board }) {
  const { view, skin } = useGame();
  const outcome = !report.battleDecided ? "" : report.battleWinner === faction ? "win" : "lose";
  const you = !view.viewer.isObserver && view.viewer.faction === faction ? "you" : "";
  const locations = view.players.some((p) => p.faction === faction) ? forceLocations(board, faction) : "";
  return (
    <div className={`side ${outcome} ${you}`} data-ref-faction={faction} data-ref-locations={locations} style={{ ["--fc" as string]: skin.factionColor(faction) }}>
      <img src={skin.factionImage(faction)} alt="" /><span className="name">{skin.faction(faction)}</span>
    </div>
  );
}

// The lines after a battle's revealed plans that state its outcome: strengths, the winner, kills,
// traitors, and what the two sides lost. These are drawn under the plans instead of as lines.
// Facts about anyone else, and anything logged before the plans, stay lines.
function outcomeFacts(report: ReportHeader, lines: LogLine[]): Set<LogLine> {
  const at = lines.findIndex((m) => m.plans != null);
  if (at < 0) return new Set();
  const plans = lines[at].plans!;
  const side = (f: Faction) => f === report.aggressor || f === report.defender;
  const fought = (h: BattleFact["hero"]) => h != null && (samePart(h, plans.aggressor.leader) || samePart(h, plans.defender.leader));
  return new Set(lines.slice(at + 1).filter((m) => {
    const f = m.fact;
    if (f == null) return false;
    switch (f.kind) {
      case "Winner": return true;
      case "Killed": return fought(f.hero) && side(f.faction);
      case "Traitor": return fought(f.hero);
      default: return side(f.faction);
    }
  }));
}

// The revealed plans, then the two strengths with the winner's bright and the loser's dimmed,
// then what each side gained and lost.
function Outcome({ report, plans, facts }: { report: ReportHeader; plans: BattlePlans; facts: BattleFact[] }) {
  const aggStrength = facts.find((f) => f.kind === "Strength" && f.faction === report.aggressor);
  const defStrength = facts.find((f) => f.kind === "Strength" && f.faction === report.defender);
  const anyAftermath = facts.some((f) => f.kind === "Killed" || f.kind === "ForcesLost" || f.kind === "Discard");
  return (
    <>
      <div className="plans">
        <Plan plan={plans.aggressor} facts={facts} />
        <div className="rule"></div>
        <Plan plan={plans.defender} facts={facts} />
      </div>
      {aggStrength && defStrength && (
        <div className="verdict">
          <Total report={report} strength={aggStrength} />
          <Total report={report} strength={defStrength} />
        </div>
      )}
      {anyAftermath && (
        <div className="aftermath">
          <Aftermath faction={report.aggressor} facts={facts} />
          <div className="rule"></div>
          <Aftermath faction={report.defender} facts={facts} />
        </div>
      )}
    </>
  );
}

function Total({ report, strength }: { report: ReportHeader; strength: BattleFact }) {
  const { skin } = useGame();
  const lost = report.battleDecided && report.battleWinner !== strength.faction;
  return <div className={`total ${lost ? "lost" : ""}`} title={`${skin.faction(strength.faction)} strength`}>{supportRound(strength.strength)}</div>;
}

// One side's spice from killed leaders, forces lost and cards discarded.
function Aftermath({ faction, facts }: { faction: Faction; facts: BattleFact[] }) {
  const { view } = useGame();
  const mine = facts.filter((f) => f.faction === faction && (f.kind === "Killed" || f.kind === "ForcesLost" || f.kind === "Discard"));
  const player = view.players.some((p) => p.faction === faction);
  return (
    <div className="side-after">
      {mine.length === 0 && <div className="quiet">no losses</div>}
      {mine.map((f, i) => {
        switch (f.kind) {
          case "Killed":
            return (
              <div key={i} className="item">
                <span className="plus">+</span><SpiceComponent amount={String(f.value)} size={22} /><span className="for">for</span>
                {f.hero && <HeroImage hero={f.hero} width={22} className="leader-disc" />}
              </div>
            );
          case "ForcesLost":
            return (
              <div key={i} className="item">
                <span className="minus">−</span>
                {f.all && <span>all</span>}
                {f.forces > 0 && player && <><SimpleNumber size={20}>{f.forces}</SimpleNumber><ForceImage faction={faction} special={false} width={20} /></>}
                {f.specialForces > 0 && player && <><SimpleNumber size={20}>{f.specialForces}</SimpleNumber><ForceImage faction={faction} special width={20} /></>}
              </div>
            );
          default:
            return <div key={i} className="item"><span className="minus">−</span>{f.card && <PartText part={f.card} />}</div>;
        }
      })}
    </div>
  );
}
