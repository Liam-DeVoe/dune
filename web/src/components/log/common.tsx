// What the report panel (ReportComponent.razor) and the phase logs (OtherComponents/Log) share:
// the plain line (ReportComponent.Line), a battle plan as the log draws it (ReportComponent.Plan),
// typed access to a line's record, and the small helpers several logs repeat (force counts,
// route hover attributes, part identity).
import { type CSSProperties, Fragment, type ReactNode } from "react";
import type { BattleFact } from "../../generated/BattleFact";
import { type Faction, FactionValue } from "../../generated/enums";
import type { LogLine } from "../../generated/LogLine";
import type { LogRecord } from "../../generated/LogRecord";
import type { Part } from "../../generated/Part";
import type { PlanSummary } from "../../generated/PlanSummary";
import type { Route } from "../../generated/Route";
import { leaderPopup, messiahPopup, spiceHtml, treacheryCardPopup } from "../../popups";
import type { Skin } from "../../skin";
import { useGame } from "../../state/context";
import { htmlEncode, supportRound } from "../../text";
import { Board } from "../board/geometry";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { Image } from "../Image";
import { PlaceChip } from "../PlaceChip";
import { RichText } from "../RichText";
import { SimpleNumber } from "../SimpleNumber";
import { SpiceComponent } from "../SpiceComponent";

/** PhaseLogBase: a main phase's visible lines, and the report's plain line for the rest. */
export interface PhaseLogProps {
  lines: LogLine[];
  line: (l: LogLine) => ReactNode;
  /** Whether these lines are the game's latest report (BlowLog.NexusBegun asks). */
  latest: boolean;
}

export type RecordOf<T extends LogRecord["type"]> = Extract<LogRecord, { type: T }>;

/** `message.Log is T t`. */
export function rec<T extends LogRecord["type"]>(l: LogLine | undefined, type: T): RecordOf<T> | null {
  return l?.record?.type === type ? (l.record as RecordOf<T>) : null;
}

export function is(l: LogLine | undefined, ...types: LogRecord["type"][]): boolean {
  return l?.record != null && types.includes(l.record.type);
}

/** C# reference equality of two game objects drawn as parts: same kind, same id. */
export function samePart(a: Part | null | undefined, b: Part | null | undefined): boolean {
  if (a == null || b == null) return false;
  if (a.t !== b.t) return false;
  if ("id" in a && "id" in b) return a.id === b.id;
  return JSON.stringify(a) === JSON.stringify(b);
}

/** `Message.ToString(skin)`: the line as plain text. */
export function plainText(parts: Part[]): string {
  return parts.map((p) => {
    switch (p.t) {
      case "text": return p.v;
      case "none": return "none";
      case "num": return String(p.v);
      case "payment": return String(p.amount);
      case "faction": return p.label;
      default: return "label" in p ? p.label : "";
    }
  }).join("");
}

/** The id of a location or territory part, or -1. */
export function placeId(p: Part | null | undefined): number {
  return p != null && (p.t === "location" || p.t === "territory") ? p.id : -1;
}

/** `PlaceChip Place=...` for a location or territory part. */
export function PlacePart({ part }: { part: Part | null | undefined }) {
  if (part?.t === "territory") return <PlaceChip place={{ territory: part.id }} />;
  if (part?.t === "location") return <PlaceChip place={{ location: part.id }} />;
  return part != null ? <RichText parts={[part]} size={20} logStyle /> : null;
}

/** `Expression(card)` in the log style: one part as ExpressionComponent draws it. */
export function PartText({ part, size = 20 }: { part: Part; size?: number }) {
  return <RichText parts={[part]} size={size} logStyle />;
}

const italic: CSSProperties = { fontStyle: "italic" };

/** ReportComponent.FontStyle. */
export function fontStyle(l: LogLine): CSSProperties | undefined {
  return l.private ? italic : undefined;
}

/** Skin.Describe of a force or special force (FactionForce / FactionSpecialForce). */
export function forceName(skin: Skin, faction: Faction, special: boolean): string {
  return skin.get(special ? "SpecialForceName_STR" : "ForceName_STR", faction);
}

/** The locations a route touches and its map arrows (ReportComponent.RouteLocations, RouteArrow). */
export function routeAttributes(board: Board, route: Route | null): { locations: string; arrow: string } {
  if (route == null) return { locations: "", arrow: "" };
  const from = route.from.map(placeId);
  const to = route.to != null ? placeId(route.to) : null;
  const locations = [...from, ...(to != null ? [to] : [])].join(",");
  const player = board.view.players.find((p) => p.faction === route.faction);
  const token = player ? board.playerTokenPosition(player.seat) : null;
  const end = to != null && board.hasCenter(to) ? board.getCenter(to) : token;
  if (end == null) return { locations, arrow: "" };
  const starts = from.filter((l) => board.hasCenter(l)).map((l) => board.getCenter(l));
  if (starts.length === 0 && token) starts.push(token);
  const arrow = starts.map((p) => `${supportRound(p.X)},${supportRound(p.Y)},${supportRound(end.X)},${supportRound(end.Y)}`).join(";");
  return { locations, arrow };
}

/** The locations holding a faction's forces (`player.ForcesOnPlanet.Keys`). */
export function forceLocations(board: Board, faction: Faction): string {
  return [...new Set(board.view.board.forces.filter((f) => f.faction === faction).map((f) => f.location))].join(",");
}

// ReportComponent.AdvantageOwner: the faction an advantage's name starts with.
const factionsByValue = (Object.entries(FactionValue) as [Faction, number][]).sort((a, b) => a[1] - b[1]).map(([f]) => f);

function advantageOwner(advantage: string): Faction {
  return factionsByValue.find((f) => f !== "None" && advantage.startsWith(f)) ?? "None";
}

// ReportComponent.WithSpice: "2 spice" as the spice amount and a lone "spice" as the icon.
function withSpice(skin: Skin, text: string): string {
  const name = skin.concept("Resource").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`\\b(?:(\\d+) )?${name}\\b(?! (?:cards?|blows?|decks?)\\b)`, "gi");
  return htmlEncode(text).replace(re, (_m, digits: string | undefined) => spiceHtml(skin, digits ?? "", 20));
}

/** ReportComponent.AsideParts and Advantage. */
function AsideParts({ parts }: { parts: Part[] }) {
  const { skin } = useGame();
  return (
    <>
      {parts.map((p, i) => {
        if (p.t === "enum" && p.ty === "FactionAdvantage") {
          const owner = advantageOwner(p.v);
          const name = owner === "None" ? "" : skin.faction(owner);
          const named = name.length > 0 && p.label.startsWith(name);
          const rest = named ? p.label.slice(name.length) : p.label;
          return <Fragment key={i}>{named && <FactionChip faction={owner} />}<span dangerouslySetInnerHTML={{ __html: withSpice(skin, rest) }} /></Fragment>;
        }
        return <PartText key={i} part={p} />;
      })}
    </>
  );
}

/** ReportComponent.HeadingIcon. */
function headingIcon(skin: Skin, icon: string): string {
  return icon === "Resource" ? skin.url(skin.str("CardBack_ResourceCard_URL")) : "";
}

function asideClass(kind: string): string {
  return kind === "Answer" ? "answer" : kind === "After" ? "after" : "";
}

/** ReportComponent.Line: one message as the log draws it when no design takes it. */
export function Line({ line: l, board }: { line: LogLine; board: Board }) {
  const { skin } = useGame();
  if (l.heading != null) {
    const url = headingIcon(skin, l.heading);
    return (
      <div className="heading">
        {url.length > 0 && <img src={url} alt="" />}
        <RichText parts={l.text} size={20} logStyle />
      </div>
    );
  }
  if (l.plans) {
    return (
      <div className="plans">
        <Plan plan={l.plans.aggressor} facts={[]} />
        <div className="rule"></div>
        <Plan plan={l.plans.defender} facts={[]} />
      </div>
    );
  }
  const aside = rec(l, "AsideLog");
  if (aside) {
    return <div className={`log-aside ${asideClass(aside.kind)}`} style={fontStyle(l)}><AsideParts parts={l.text} /></div>;
  }
  if (l.route) {
    const r = routeAttributes(board, l.route);
    return (
      <div style={{ ...fontStyle(l), ["--fc" as string]: skin.factionColor(l.route.faction) }} data-ref-locations={r.locations} data-ref-arrow={r.arrow}>
        <RichText parts={l.text} size={20} logStyle />
      </div>
    );
  }
  return <div style={fontStyle(l)}><RichText parts={l.text} size={20} logStyle /></div>;
}

/** `Image Shown=hero`: a leader's disc (or the Kwisatz Haderach) with its popover. */
export function HeroImage({ hero, width, className = "" }: { hero: Part; width: number; className?: string }) {
  const { skin } = useGame();
  if (hero.t === "leader") {
    return <Image src={skin.personImage(hero.id)} alt={hero.label} width={width} className={className} noShadow popover={leaderPopup(skin, hero.id, hero.skill)} />;
  }
  // A Cheap Hero is a treachery card.
  if (hero.t === "treacheryCard") {
    return <Image src={skin.cardImage(hero.id)} alt={hero.label} width={width} className={className} noShadow popover={treacheryCardPopup(skin, hero.id)} />;
  }
  const label = "label" in hero ? hero.label : "";
  return <Image src={skin.url(skin.str("Messiah_URL"))} alt={label} width={width} className={className} noShadow popover={messiahPopup(skin)} />;
}

/** `Image Shown=card`: a treachery card's art with its popover. */
export function CardImage({ card, width }: { card: Part; width: number }) {
  const { skin } = useGame();
  if (card.t !== "treacheryCard") return null;
  return <Image src={skin.cardImage(card.id)} alt={card.label} width={width} noShadow popover={treacheryCardPopup(skin, card.id)} />;
}

/** `Image Shown=player.Force` (or SpecialForce): the token, named on hover. */
export function ForceImage({ faction, special, width }: { faction: Faction; special: boolean; width: number }) {
  const { skin } = useGame();
  const name = forceName(skin, faction, special);
  const src = skin.url(skin.get(special ? "FactionSpecialForceImage_URL" : "FactionForceImage_URL", faction));
  const starred = special && (faction === "Fremen" || faction === "Emperor" || faction === "Ixian");
  return <Image src={src} alt={name} width={width} noShadow tooltip={name} starred={starred} />;
}

const skullPath = "M8 1.5C4.7 1.5 2.5 3.8 2.5 6.8c0 1.8.8 3 2 3.8v2.2c0 .5.4.9.9.9h.6v-1.4h1v1.4h2v-1.4h1v1.4h.6c.5 0 .9-.4.9-.9v-2.2c1.2-.8 2-2 2-3.8 0-3-2.2-5.3-5.5-5.3zM5.7 9a1.4 1.4 0 1 1 0-2.8 1.4 1.4 0 0 1 0 2.8zm4.6 0a1.4 1.4 0 1 1 0-2.8 1.4 1.4 0 0 1 0 2.8z";

/** ReportComponent.Plan: one side of the revealed plans, laid out like the pieces on the table. */
export function Plan({ plan, facts }: { plan: PlanSummary; facts: BattleFact[] }) {
  const { skin } = useGame();
  const killed = facts.find((f) => f.kind === "Killed" && plan.leader != null && samePart(f.hero, plan.leader));
  const traitor = facts.find((f) => f.kind === "Traitor" && plan.leader != null && samePart(f.hero, plan.leader));
  const messiah = skin.concept("Messiah");
  return (
    <div className="plan" data-ref-faction={plan.faction}>
      <div className={`plan-leader ${killed || traitor ? "fallen" : ""}`}>
        {plan.leader != null ? <HeroImage hero={plan.leader} width={58} /> : <div className="empty" title="No leader"></div>}
        {plan.messiah && plan.leader != null && (
          <span className="messiah" title={messiah}><img src={skin.url(skin.str("Messiah_URL"))} alt={messiah} />+2</span>
        )}
        {traitor
          ? <span className="traitor" title={`Traitor for ${skin.faction(traitor.faction)}`}>Traitor</span>
          : killed && (
            <span className="skull" title={`Killed by ${plainText(killed.cause ?? [])}`}><svg viewBox="0 0 16 16" aria-hidden="true"><path d={skullPath} /></svg></span>
          )}
      </div>
      <div className="dial">
        <SimpleNumber size={30}>{supportRound(plan.dial)}</SimpleNumber>
        {plan.spice != null && plan.spice > 0 && <SpiceComponent amount={String(plan.spice)} size={22} />}
      </div>
      {plan.allyContribution > 0 && (
        <div className="ally">+<SpiceComponent amount={String(plan.allyContribution)} size={18} /> from <FactionChip faction={plan.ally} /></div>
      )}
      <div className="cards">
        <CardSlot card={plan.weapon} none="no weapon" />
        <CardSlot card={plan.defense} none="no defense" />
      </div>
    </div>
  );
}

function CardSlot({ card, none }: { card: Part | null; none: string }) {
  return (
    <div className="card-slot">
      {card != null ? (
        <>
          <CardImage card={card} width={40} />
          <div className="cap">{"label" in card ? card.label : ""}</div>
        </>
      ) : (
        <>
          <div className="empty"></div>
          <div className="cap none">{none}</div>
        </>
      )}
    </div>
  );
}

/** "[n][force] [n][special force]": a count and a token per kind, each in a span of class `wrap`. */
export function ForceCounts({ faction, forces, specialForces, wrap, size = 20 }: {
  faction: Faction; forces: number; specialForces: number; wrap: string; size?: number;
}) {
  return (
    <>
      {forces > 0 && <span className={wrap}><SimpleNumber size={size}>{forces}</SimpleNumber><ForceToken faction={faction} size={size} /></span>}
      {specialForces > 0 && <span className={wrap}><SimpleNumber size={size}>{specialForces}</SimpleNumber><ForceToken faction={faction} special size={size} /></span>}
    </>
  );
}
