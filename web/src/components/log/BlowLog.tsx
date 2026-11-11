// OtherComponents/Log/BlowLog.razor: each spice card drawn is shown as the card, with its amount
// and place beside it. A worm is an aside holding what it devoured, and the Nexus after it an
// aside with the alliance changes and the Fremen ride. An alliance change is two faction chips
// with a glyph between them; an offer that was accepted is folded into its alliance. With
// Increased Spice Flow a small caps label heads each deck. Every other message is a plain line in
// its place.
import { Fragment, type ReactNode } from "react";
import type { Faction, NexusAllianceChange, Phase } from "../../generated/enums";
import type { LogLine } from "../../generated/LogLine";
import type { NexusAlliance } from "../../generated/NexusAlliance";
import { useGame } from "../../state/context";
import { useBoardGeometry } from "../shipmove/common";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { MonsterIcon } from "../MonsterIcon";
import { RichText } from "../RichText";
import { SimpleNumber } from "../SimpleNumber";
import { SpiceComponent } from "../SpiceComponent";
import { is, type PhaseLogProps, PlacePart, placeId, rec, type RecordOf, routeAttributes } from "./common";

type Item =
  | { kind: "line"; line: LogLine }
  | { kind: "deck"; deck: RecordOf<"BlowDeckLog"> }
  | { kind: "card"; line: LogLine; card: RecordOf<"BlowCardLog"> }
  | { kind: "note"; line: LogLine }
  | { kind: "worm"; territory: number | null; lines: LogLine[] }
  | { kind: "nexus"; lines: LogLine[] };

type WormItem = Extract<Item, { kind: "worm" }>;
type NexusItem = Extract<Item, { kind: "nexus" }>;

const wormTypes = ["WormAppearedLog", "WormSentLog", "WormDevouredForcesLog", "WormSurvivedLog", "WormDevouredSpiceLog"] as const;

function isWorm(l: LogLine): boolean {
  return is(l, ...wormTypes);
}

// Whether another Nexus line comes after line i before the next card or worm.
function nexusContinues(section: LogLine[], i: number): boolean {
  const next = section.slice(i + 1).find((m) => is(m, "NexusLog", "BlowCardLog") || isWorm(m));
  return next != null && is(next, "NexusLog");
}

// The lines split at each deck heading.
function sections(lines: LogLine[]): LogLine[][] {
  const result: LogLine[][] = [[]];
  for (const l of lines) {
    if (is(l, "BlowDeckLog") && result[result.length - 1].length > 0) result.push([]);
    result[result.length - 1].push(l);
  }
  return result;
}

const drawingPhases: Phase[] = ["Thumper", "BlowA", "BlowB", "HarvesterA", "HarvesterB", "FremenSendingMonsterA", "FremenSendingMonsterB"];

// The lines as the design's pieces, in the game's order. `nexusBegun(last)` says whether a deck's
// Nexus has begun (BlowLog.NexusBegun).
function buildItems(lines: LogLine[], nexusBegun: (lastSection: boolean) => boolean): Item[] {
  const items: Item[] = [];
  const all = sections(lines);
  all.forEach((section, s) => {
    const firstNexus = section.findIndex((m) => is(m, "NexusLog"));
    const hadWorm = section.some((m) => is(m, "WormAppearedLog", "WormSentLog"));
    let lastBlowItem = -1;
    let worm: WormItem | null = null;
    let nexus: NexusItem | null = null;

    for (let i = 0; i < section.length; i++) {
      const message = section[i];

      // A Nexus line, or a line logged between two of them.
      if (is(message, "NexusLog") || (nexus != null && !is(message, "BlowCardLog") && !isWorm(message) && nexusContinues(section, i))) {
        if (nexus == null) {
          nexus = { kind: "nexus", lines: [] };
          items.push(nexus);
        }
        nexus.lines.push(message);
        worm = null;
        continue;
      }

      const r = message.record;
      switch (r?.type) {
        case "BlowDeckLog":
          items.push({ kind: "deck", deck: r });
          break;
        case "BlowCardLog":
          items.push({ kind: "card", line: message, card: r });
          lastBlowItem = items.length - 1;
          worm = null;
          nexus = null;
          break;
        case "WormAppearedLog":
          worm = { kind: "worm", territory: placeId(r.territory), lines: [message] };
          items.push(worm);
          lastBlowItem = items.length - 1;
          nexus = null;
          break;
        case "WormSentLog":
          // The Fremen choosing where a later worm goes, right after the game says it appears
          // again: one worm, one aside, and the send line stands in for "appears a second time".
          if (worm != null && items[items.length - 1] === worm && is(worm.lines[worm.lines.length - 1], "WormAppearedLog")) {
            const lines2: LogLine[] = [...worm.lines];
            lines2[lines2.length - 1] = message;
            worm = { kind: "worm", territory: placeId(r.territory), lines: lines2 };
            items[items.length - 1] = worm;
          } else {
            worm = { kind: "worm", territory: placeId(r.territory), lines: [message] };
            items.push(worm);
            lastBlowItem = items.length - 1;
            nexus = null;
          }
          break;
        case "WormDevouredForcesLog":
        case "WormSurvivedLog":
        case "WormDevouredSpiceLog":
          if (worm == null || items[items.length - 1] !== worm) {
            worm = { kind: "worm", territory: null, lines: [] };
            items.push(worm);
          }
          worm.lines.push(message);
          lastBlowItem = items.length - 1;
          break;
        case "WormIgnoredLog":
        case "BlowReshuffleLog":
          items.push({ kind: "note", line: message });
          break;
        default:
          items.push({ kind: "line", line: message });
          worm = null;
          break;
      }
    }

    if (hadWorm && firstNexus < 0 && lastBlowItem >= 0 && nexusBegun(s === all.length - 1)) {
      items.splice(lastBlowItem + 1, 0, { kind: "nexus", lines: [] });
    }
  });
  return items;
}

function samePair(a: NexusAlliance, b: NexusAlliance): boolean {
  return (a.from === b.from && a.to === b.to) || (a.from === b.to && a.to === b.from);
}

// An offer that an alliance later in the same Nexus answered. The alliance's row stands for it.
function accepted(message: LogLine, nexus: LogLine[]): boolean {
  const offer = rec(message, "NexusLog")?.alliance;
  if (offer?.change !== "Offered") return false;
  return nexus.slice(nexus.indexOf(message) + 1).some((m) => {
    const made = rec(m, "NexusLog")?.alliance;
    return made?.change === "Made" && samePair(made, offer);
  });
}

// Who made the offer an alliance accepted. An alliance without an offer keeps the game's order.
function offerer(made: NexusAlliance, nexus: LogLine[]): Faction {
  const offer = nexus.map((m) => rec(m, "NexusLog")?.alliance).find((a) => a != null && a.change === "Offered" && samePair(a, made));
  return offer?.from ?? made.from;
}

// Forces as number and token, special forces after.
function Forces({ faction, forces, specialForces }: { faction: Faction; forces: number; specialForces: number }) {
  return (
    <span className="blow-forces">
      {forces > 0 && <><SimpleNumber size={20}>{forces}</SimpleNumber><ForceToken faction={faction} size={20} /></>}
      {specialForces > 0 && <><SimpleNumber size={20}>{specialForces}</SimpleNumber><ForceToken faction={faction} special size={20} /></>}
    </span>
  );
}

function AllianceGlyph({ change }: { change: NexusAllianceChange }) {
  switch (change) {
    case "Made":
      return (
        <svg className="nexus-glyph" viewBox="0 0 22 22" role="img" aria-label="allied with"><title>allied with</title>
          <circle cx="8" cy="11" r="5" /><path d="M14 6a5 5 0 1 1 0 10" /><path d="M14 16a5 5 0 0 1-4.6-3" /><path d="M9.4 9A5 5 0 0 1 14 6" />
        </svg>
      );
    case "Offered":
      return (
        <svg className="nexus-glyph" viewBox="0 0 22 22" role="img" aria-label="offer an alliance to"><title>offer an alliance to</title>
          <path d="M3 11h11" strokeDasharray="2.2 2.6" /><path d="M13 6.5l5 4.5-5 4.5" />
        </svg>
      );
    default:
      return (
        <svg className="nexus-glyph" viewBox="0 0 22 22" role="img" aria-label="ended their alliance with"><title>ended their alliance with</title>
          <path d="M8.5 7.2A4.2 4.2 0 1 0 8.5 14.8" /><path d="M13.5 7.2A4.2 4.2 0 1 1 13.5 14.8" /><path d="M11 5.5v2.2M11 14.3v2.2M11 10v2" />
        </svg>
      );
  }
}

function AllianceStep({ change, from, to }: { change: NexusAllianceChange; from: Faction; to: Faction }) {
  return (
    <div className="blow-line nexus-step">
      <FactionChip faction={from} />
      <AllianceGlyph change={change} />
      <FactionChip faction={to} />
    </div>
  );
}

export function BlowLog({ lines, line, latest }: PhaseLogProps) {
  const { view, skin } = useGame();
  const board = useBoardGeometry();

  // Whether a deck's Nexus has started. Only the deck being drawn in the phase going on now can
  // still be waiting for its card or a worm sent by the Fremen.
  const nexusBegun = (lastSection: boolean) => {
    if (!lastSection || view.mainPhase !== "Blow") return true;
    if (lines.length === 0 || !latest) return true;
    return !drawingPhases.includes(view.phase);
  };

  const card = (c: RecordOf<"BlowCardLog">): ReactNode => (
    <div className="blow-card" data-ref-locations={placeId(c.location)}>
      <img className={`card ${c.lost ? "lost" : ""}`} src={skin.url(skin.get("ResourceCardImage_URL", c.card.t === "resourceCard" ? c.card.id : -1))}
        alt={"label" in c.card ? c.card.label : ""} title={"label" in c.card ? c.card.label : ""} />
      <div>
        <div className="big">
          <SpiceComponent amount={String(c.amount)} size={34} />
          {!c.lost && <span>detected in</span>}
        </div>
        <div className="where">
          {c.lost
            ? <><span>in </span><PlacePart part={c.location} /><span> is lost in the storm</span></>
            : <><PlacePart part={c.location} />{c.doubled && <span>, doubled by {skin.concept("BabyMonster")}</span>}</>}
        </div>
      </div>
    </div>
  );

  // A line inside a worm's aside. The aside's first line names the place.
  const wormLine = (message: LogLine, key: number): ReactNode => {
    const r = message.record;
    switch (r?.type) {
      case "WormDevouredForcesLog":
        return <div key={key} className="blow-line"><MonsterIcon /> devours <FactionChip faction={r.faction} /> <Forces faction={r.faction} forces={r.forces} specialForces={r.specialForces} /></div>;
      case "WormSurvivedLog":
        return <div key={key} className="blow-line"><FactionChip faction={r.faction} /> survive <MonsterIcon /></div>;
      case "WormDevouredSpiceLog":
        return <div key={key} className="blow-line"><MonsterIcon /> devours <SpiceComponent amount={String(r.amount)} size={20} /></div>;
      default:
        return <div key={key} className="blow-line"><RichText parts={message.text} size={20} logStyle /></div>;
    }
  };

  // A line inside the Nexus aside.
  const nexusLine = (message: LogLine, nexus: LogLine[], key: number): ReactNode => {
    const n = rec(message, "NexusLog");
    if (n?.alliance?.change === "Made") {
      const first = offerer(n.alliance, nexus);
      return <AllianceStep key={key} change="Made" from={first} to={first === n.alliance.from ? n.alliance.to : n.alliance.from} />;
    }
    if (n?.alliance) return <AllianceStep key={key} change={n.alliance.change} from={n.alliance.from} to={n.alliance.to} />;
    if (n?.ride) {
      const ride = n.ride;
      const r = routeAttributes(board, message.route);
      return (
        <div key={key} className="blow-line" style={{ ["--fc" as string]: skin.factionColor(ride.faction) }} data-ref-locations={r.locations} data-ref-arrow={r.arrow}>
          <FactionChip faction={ride.faction} /> ride <Forces faction={ride.faction} forces={ride.forces} specialForces={ride.specialForces} />
          {ride.from != null
            ? <><span>from </span><PlacePart part={ride.from} /><span> to </span><PlacePart part={ride.to} /></>
            : <><span>from their reserves to </span><PlacePart part={ride.to} /></>}
        </div>
      );
    }
    if (n) return <div key={key} className="blow-line"><RichText parts={message.text} size={20} logStyle /></div>;
    return <Fragment key={key}>{line(message)}</Fragment>;
  };

  const territoryLocations = (t: number | null) => (t == null ? "" : (skin.territoryInfo(t)?.locations ?? []).join(","));

  return (
    <>
      {buildItems(lines, nexusBegun).map((item, i) => {
        switch (item.kind) {
          case "deck":
            return <div key={i} className="log-label blow-deck">Deck {item.deck.deck}</div>;
          case "card":
            return <Fragment key={i}>{card(item.card)}</Fragment>;
          case "note":
            return <div key={i} className="blow-note"><RichText parts={item.line.text} size={18} logStyle /></div>;
          case "worm":
            return (
              <div key={i} className="log-aside blow-worm" data-ref-locations={territoryLocations(item.territory)}>
                {item.lines.map((m, j) => wormLine(m, j))}
              </div>
            );
          case "nexus":
            return (
              <div key={i} className="log-aside blow-nexus">
                <div className="log-label">Nexus</div>
                {item.lines.length === 0 && <div className="blow-none">No alliance changes</div>}
                {item.lines.filter((m) => !accepted(m, item.lines)).map((m, j) => nexusLine(m, item.lines, j))}
              </div>
            );
          case "line":
            return <Fragment key={i}>{line(item.line)}</Fragment>;
        }
      })}
    </>
  );
}
