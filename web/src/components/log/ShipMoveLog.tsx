// OtherComponents/Log/ShipMoveLog.razor: one block per faction turn, in acting order. Each block
// has a Ship (or Rally) row and a Move row, "passed" when the faction did nothing; a Hajr move
// gets its own row. Bene Gesserit answers and forces lost follow the row that caused them, marked
// +. The game logs storm losses before the shipment or move that caused them, so they wait for
// it. Lines the design doesn't draw keep their place: inside the block while the turn is under
// way, between blocks otherwise.
import { Fragment, type ReactNode } from "react";
import type { Faction } from "../../generated/enums";
import type { LogLine } from "../../generated/LogLine";
import type { Part } from "../../generated/Part";
import type { Route } from "../../generated/Route";
import { useGame } from "../../state/context";
import { type Board } from "../board/geometry";
import { useBoardGeometry } from "../shipmove/common";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { RichText } from "../RichText";
import { SpiceComponent } from "../SpiceComponent";
import { ForceCounts, PartText, type PhaseLogProps, PlacePart, rec, type RecordOf, routeAttributes, samePart } from "./common";

type RowKind = "Ship" | "Move" | "Passed" | "Plus" | "Other";

interface RowData { kind: RowKind; lines: LogLine[] }

interface Turn {
  kind: "turn";
  faction: Faction;
  rows: RowData[];
  // The faction's move is over: lines that aren't theirs go between blocks.
  closed: boolean;
  // "pass movement" was logged before the shipment: the Move row goes last.
  passPending: boolean;
}

interface Delay { kind: "delay"; lines: LogLine[] }

type Item = Turn | Delay | { kind: "line"; line: LogLine };

// The game's "X discard [card]" line (faction None: anyone).
function isDiscardOf(m: LogLine, faction: Faction, card: (c: Part) => boolean): boolean {
  const f = m.fact;
  return f?.kind === "Discard" && f.card != null && card(f.card) && (faction === "None" || f.faction === faction);
}

function factionOf(m: LogLine): Faction {
  return rec(m, "MovePassed")?.faction ?? rec(m, "ShipMoveLoss")?.faction ?? "None";
}

function moveOf(row: RowData): RecordOf<"MoveLogged"> {
  return rec(row.lines[0], "MoveLogged")!;
}

// The log as plain lines, Guild delays and turn blocks.
function build(lines: LogLine[], isCaravan: (c: Part) => boolean): Item[] {
  const items: Item[] = [];
  let turn: Turn | null = null;
  const storm: LogLine[] = [];
  const early: LogLine[] = [];
  let afterMove = false;
  const last = <T,>(a: T[]): T | undefined => a[a.length - 1];

  const flushStorm = () => {
    if (turn != null) for (const m of storm) turn.rows.push({ kind: "Plus", lines: [m] });
    else for (const m of storm) items.push({ kind: "line", line: m });
    storm.length = 0;
  };

  const close = () => {
    if (turn == null) return;
    if (turn.passPending && !turn.rows.some((r) => r.kind === "Move" || r.kind === "Passed")) turn.rows.push({ kind: "Passed", lines: [] });
    turn.passPending = false;
  };

  const newTurn = (faction: Faction): Turn => ({ kind: "turn", faction, rows: [], closed: false, passPending: false });

  for (const message of lines) {
    // Storm losses on the way wait until every line of the move is in.
    if (afterMove) {
      const next = rec(message, "MoveLogged");
      const lastRow: RowData | undefined = turn ? last(turn.rows) : undefined;
      if (!(next && lastRow?.kind === "Move" && moveOf(lastRow).move === next.move)) {
        afterMove = false;
        flushStorm();
      }
    }

    const r = message.record;
    switch (r?.type) {
      case "GuildDelayed": {
        close();
        turn = null;
        const lastItem = last(items);
        if (lastItem?.kind === "delay") lastItem.lines.push(message);
        else items.push({ kind: "delay", lines: [message] });
        break;
      }
      case "ShipLogged": {
        close();
        // The Karama card shows in the Ship row; its discard line goes.
        const lastItem = last(items);
        if (r.karama != null && lastItem?.kind === "line" && isDiscardOf(lastItem.line, "None", (c) => samePart(c, r.karama))) items.pop();
        turn = newTurn(r.faction);
        items.push(turn);
        turn.rows.push({ kind: "Ship", lines: [message] });
        flushStorm();
        for (const m of early.filter((m) => factionOf(m) === r.faction)) {
          early.splice(early.indexOf(m), 1);
          if (rec(m, "MovePassed")) turn.passPending = turn.closed = true;
          else turn.rows.push({ kind: "Plus", lines: [m] });
        }
        break;
      }
      case "MoveLogged": {
        // The HAJR label names the card; its discard line goes.
        const lastRow: RowData | undefined = turn ? last(turn.rows) : undefined;
        const lastItem = last(items);
        if (r.hajr && turn && lastRow?.kind === "Other" && isDiscardOf(lastRow.lines[0], r.faction, isCaravan)) turn.rows.splice(turn.rows.indexOf(lastRow), 1);
        else if (r.hajr && lastItem?.kind === "line" && isDiscardOf(lastItem.line, r.faction, isCaravan)) items.pop();

        if (turn == null || turn.faction !== r.faction) {
          close();
          turn = newTurn(r.faction);
          items.push(turn);
        }

        const previous = last(turn.rows);
        if (previous?.kind === "Move" && moveOf(previous).move === r.move) previous.lines.push(message);
        else turn.rows.push({ kind: "Move", lines: [message] });

        if (!r.hajr) turn.closed = true;
        afterMove = true;
        break;
      }
      case "MovePassed":
        if (turn?.faction === r.faction) {
          turn.rows.push({ kind: "Passed", lines: [] });
          turn.closed = true;
        } else {
          early.push(message);
        }
        break;
      case "BeneGesseritReaction":
        if (turn != null) turn.rows.push({ kind: "Plus", lines: [message] });
        else other(message);
        break;
      case "ShipMoveLoss":
        if (r.storm) storm.push(message);
        else if (turn?.faction === r.faction) turn.rows.push({ kind: "Plus", lines: [message] });
        else early.push(message);
        break;
      default:
        other(message);
        break;
    }
  }

  function other(message: LogLine) {
    if (turn != null && !turn.closed) turn.rows.push({ kind: "Other", lines: [message] });
    else items.push({ kind: "line", line: message });
  }

  close();
  flushStorm();
  for (const m of early) items.push({ kind: "line", line: m });
  return items;
}

// The hover attributes of the report's route lines.
function routeProps(board: Board, route: Route | null) {
  if (route == null) return {};
  const r = routeAttributes(board, route);
  return { style: { ["--fc" as string]: board.skin.factionColor(route.faction) }, "data-ref-locations": r.locations, "data-ref-arrow": r.arrow };
}

function Forces({ faction, forces, specialForces }: { faction: Faction; forces: number; specialForces: number }) {
  return <ForceCounts faction={faction} forces={forces} specialForces={specialForces} wrap="forces" />;
}

function Card({ card }: { card: Part | null }) {
  const { skin } = useGame();
  return card != null ? <PartText part={card} /> : <span className="fw-bold">{skin.enumValue("TreacheryCardType", "Karma")}</span>;
}

function ShipRow({ faction, message, board }: { faction: Faction; message: LogLine; board: Board }) {
  const s = rec(message, "ShipLogged")!;
  const rally = s.kind === "Normal" && s.cost === 0;
  let content: ReactNode;
  if (s.plain) {
    content = <RichText parts={message.text} size={20} logStyle />;
  } else if (s.kind === "Passed") {
    content = s.blocked ? <><span>blocked by</span><Card card={s.blockedBy} /></> : <span>passed</span>;
  } else {
    content = (
      <>
        <Forces faction={faction} forces={s.forces} specialForces={s.specialForces} />
        {s.from != null && <PlacePart part={s.from} />}
        <span>→</span>
        {s.kind === "BackToReserves" ? <span>reserves</span> : s.to != null && <PlacePart part={s.to} />}
        {s.cost > 0 && (
          <>
            <SpiceComponent amount={String(s.cost)} />
            {s.allyShare >= s.cost
              ? <><span className="small">paid by</span><FactionChip faction={s.ally} /></>
              : s.allyShare > 0 && <><span className="small">(<SpiceComponent amount={String(s.allyShare)} /> paid by</span><FactionChip faction={s.ally} /><span className="small">)</span></>}
          </>
        )}
        {s.guildPaid && <><span className="small">→</span><FactionChip faction="Guild" /></>}
        {s.karama != null && <Card card={s.karama} />}
      </>
    );
  }
  return (
    <>
      <span className="log-label">{rally ? "Rally" : "Ship"}</span>
      <span className="route" {...routeProps(board, message.route)}>{content}</span>
    </>
  );
}

function MoveRow({ faction, lines, board }: { faction: Faction; lines: LogLine[]; board: Board }) {
  const moves = lines.map((m) => rec(m, "MoveLogged")!);
  const first = moves[0];
  const route: Route = { faction, from: lines.flatMap((m) => m.route?.from ?? []), to: first.to };
  return (
    <>
      <span className="log-label">{first.hajr ? "Hajr" : "Move"}</span>
      <span className="route" {...routeProps(board, route)}>
        {moves.map((m, i) => (
          <Fragment key={i}>
            {i > 0 && <span>+</span>}
            <Forces faction={faction} forces={m.forces} specialForces={m.specialForces} />
            <PlacePart part={m.from} />
          </Fragment>
        ))}
        <span>→</span>
        <PlacePart part={first.to} />
        {first.asAdvisors != null && <><span>as</span><ForceToken faction={faction} special={first.asAdvisors} size={20} /></>}
      </span>
    </>
  );
}

function PlusText({ message }: { message: LogLine }) {
  const r = message.record;
  if (r?.type === "BeneGesseritReaction") {
    if (r.kind === "Accompany") return <><FactionChip faction="BeneGesserit" /><span>accompany to</span><PlacePart part={r.place?.[0]} /></>;
    if (r.kind === "DontAccompany") return <><FactionChip faction="BeneGesserit" /><span>don't accompany</span></>;
    return (
      <>
        <FactionChip faction="BeneGesserit" />
        <span>{r.kind === "Become" ? "become" : "stay as"}</span>
        <ForceToken faction="BeneGesserit" special={r.advisors} size={20} />
        <span>in</span>
        <PlacePart part={r.place?.[0]} />
      </>
    );
  }
  if (r?.type === "ShipMoveLoss") {
    if (r.storm) {
      return (
        <>
          <FactionChip faction={r.faction} />
          <Forces faction={r.faction} forces={r.forces} specialForces={0} />
          <span>are killed by the storm{r.travelling ? " while travelling" : ""}</span>
        </>
      );
    }
    return (
      <>
        <span>All</span>
        <FactionChip faction={r.faction} />
        <span>forces in</span>
        <PlacePart part={r.territory} />
        <span>were killed due to ally presence</span>
      </>
    );
  }
  return <RichText parts={message.text} size={20} logStyle />;
}

export function ShipMoveLog({ lines, line }: PhaseLogProps) {
  const { skin } = useGame();
  const board = useBoardGeometry();
  const isCaravan = (c: Part) => c.t === "treacheryCard" && skin.cardType(c.id) === "Caravan";

  const row = (faction: Faction, r: RowData, key: number): ReactNode => {
    switch (r.kind) {
      case "Ship": return <ShipRow key={key} faction={faction} message={r.lines[0]} board={board} />;
      case "Move": return <MoveRow key={key} faction={faction} lines={r.lines} board={board} />;
      case "Passed": return <Fragment key={key}><span className="log-label">Move</span><span className="route">passed</span></Fragment>;
      case "Plus": return <Fragment key={key}><span className="log-label plus">+</span><span className="route"><PlusText message={r.lines[0]} /></span></Fragment>;
      default: return <div key={key} className="other">{line(r.lines[0])}</div>;
    }
  };

  return (
    <>
      {build(lines, isCaravan).map((item, i) => {
        switch (item.kind) {
          case "line":
            return <Fragment key={i}>{line(item.line)}</Fragment>;
          case "delay":
            return (
              <div key={i} className="shipmove-delay">
                <RichText parts={item.lines[0].text} size={20} logStyle />
                {item.lines.length > 1 && <span>×{item.lines.length}</span>}
              </div>
            );
          case "turn":
            return (
              <div key={i} className="log-box shipmove-turn">
                <div className="head">
                  <FactionChip faction={item.faction} />
                </div>
                <div className="rows">
                  {item.rows.map((r, j) => row(item.faction, r, j))}
                </div>
              </div>
            );
        }
      })}
    </>
  );
}
