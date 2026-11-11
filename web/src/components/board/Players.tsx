// The ring around the planet: MapComponents/Players.razor (each seat's token, ally, homeworlds and
// reserves, tech tokens, skilled leaders, ambassadors, No-Field, spice to collect, name) and
// Bids.razor (each bidder's bid under their token).
import type React from "react";
import { PhaseValue } from "../../generated/enums";
import type { PublicPlayer } from "../../generated/PublicPlayer";
import { f, SECTORS, TWO_PI, useBoard, type Board, type Point } from "./geometry";
import { Circle, MapForce, MapImage, MapText } from "./primitives";
import { useMapClick } from "./mapInteraction";
import * as pop from "./popups";

// Players.Pos: a point on the ring, minus a token's radius.
function pos(b: Board, positionOnCircle: number, radiusOfToken = 0): Point {
  const mapRadius = f(f(1.06) * b.mapRadius);
  return {
    X: b.center.X + f(Math.cos(positionOnCircle) * mapRadius) - radiusOfToken,
    Y: b.center.Y + f(Math.sin(positionOnCircle) * mapRadius) - radiusOfToken,
  };
}

interface Reserve { x: number; y: number; special: boolean; amount: number; p: PublicPlayer }

export function Players() {
  const b = useBoard();
  const click = useMapClick();
  const v = b.view;
  if (PhaseValue[v.phase] <= PhaseValue.SelectingFactions) return <div />;
  const tr = b.tokenRadius;
  const fr = b.forceRadius;
  const highlighted = new Set(v.board.highlightedTerritories);
  const out: React.ReactNode[] = [];
  for (const p of v.players) {
    const k = p.faction;
    const reserves: Reserve[] = [];
    const radiusModifier = v.status.waitingFor.includes(p.faction) ? f(1.5) : 1;
    const position = b.playerTokenPosition(p.seat);
    const x = position.X - f(radiusModifier * tr);
    const y = position.Y - f(radiusModifier * tr);
    const tokenDiameter = f(f(radiusModifier * tr) * 2);
    const positionSector = f(Math.floor(f(f(p.seat * SECTORS) / v.maximumPlayers)));

    out.push(<Circle key={`${k}c`} x={position.X} y={position.Y} radius={f(f(1.35) * tr)} fillColor={b.s("TABLEPOSITION_BACKGROUNDCOLOR")} />);
    out.push(<MapImage key={`${k}t`} x={x} y={y} diameter={tokenDiameter} src={b.factionTableImage(p.faction)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
      popover={() => pop.faction(b, p)} popoverStyle="" />);
    if (p.ally !== "None") {
      const ally = v.players.find((q) => q.faction === p.ally);
      out.push(<MapImage key={`${k}a`} x={position.X} y={position.Y + f(f(0.25) * tr)} diameter={f(f(1.2) * tr)} src={b.factionTableImage(p.ally)}
        shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} popover={ally ? () => pop.faction(b, ally) : ""} popoverStyle="" />);
    }

    let poc = f(f(TWO_PI / 180) + f(f(f(4.5) - positionSector) * TWO_PI) / SECTORS);

    for (const hw of p.homeworlds) {
      poc = f(poc + f(0.06));
      const at = pos(b, poc, tr);
      const info = b.skin.statics.homeworlds.find((h) => h.world === hw.world);
      const territory = b.territoryOf(hw.location);
      if (b.applicable("Homeworlds")) {
        out.push(<MapImage key={`${k}hw${hw.world}`} x={at.X} y={at.Y} diameter={2 * tr} src={b.url("HomeWorldImage_URL", hw.world)} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
          popover={() => pop.homeworld(b, hw.world)} onClick={(e) => click(hw.location, e)} popoverStyle="" />);
        const mark = { x: at.X + f(f(0.75) * tr), y: at.Y + f(f(1.5) * tr), diameter: f(f(0.5) * tr), shadowColor: "white", shadowX: 0.5, shadowY: 0.5, shadowBlur: 0.5 };
        if (hw.occupier != null) {
          out.push(<MapImage key={`${k}occ${hw.world}`} {...mark} src={b.factionFaceDownImage(hw.occupier)} popover={`Occupied by ${b.skin.faction(hw.occupier)}`} popoverAsTooltip />);
        } else if (hw.highThreshold) {
          out.push(<MapImage key={`${k}hi${hw.world}`} {...mark} src={b.plainUrl("HighThreshold_URL")} popover="High Threshold" popoverAsTooltip />);
        } else if (hw.lowThreshold) {
          out.push(<MapImage key={`${k}lo${hw.world}`} {...mark} src={b.plainUrl("LowThreshold_URL")} popover="Low Threshold" popoverAsTooltip />);
        }
      } else {
        out.push(<MapImage key={`${k}hw${hw.world}`} x={at.X} y={at.Y} diameter={2 * tr} src={b.url("HomeWorldImage_URL", hw.world)} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
          popover={`${b.skin.faction(info?.faction ?? p.faction)} Reserves`} popoverAsTooltip />);
      }
      if (highlighted.has(territory)) {
        out.push(<Circle key={`${k}hwc${hw.world}`} x={at.X + tr} y={at.Y + tr} radius={tr} fillColor={b.s("HIGHLIGHT_OVERLAY_COLOR")} strokeWidth={1} strokeColor="red" />);
      }
      const battalions = v.board.forces.filter((bt) => bt.location === hw.location);
      const vertical = battalions.length * fr;
      let reservesY = at.Y + f(0.5 * (2 * tr - vertical));
      for (const bt of battalions) {
        const horizontal = (bt.forces > 0 && bt.specialForces > 0 ? 2 : 1) * fr;
        let reservesX = at.X + f(0.5 * (2 * tr - horizontal));
        const bp = v.players.find((q) => q.faction === bt.faction) ?? p;
        if (bt.forces > 0) {
          reserves.push({ x: reservesX, y: reservesY, special: false, amount: bt.forces, p: bp });
          reservesX += fr;
        }
        if (bt.specialForces > 0) reserves.push({ x: reservesX, y: reservesY, special: true, amount: bt.specialForces, p: bp });
        reservesY += fr;
      }
      poc = f(poc + f(0.02));
    }

    for (const tt of p.techTokens) {
      poc = f(poc + f(TWO_PI / 90));
      const at = pos(b, poc, tr);
      out.push(<MapImage key={`${k}tt${tt}`} x={at.X} y={at.Y} diameter={2 * tr} src={b.url("TechTokenImage_URL", tt)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
        popover={() => pop.techToken(b, tt)} popoverStyle="" />);
    }

    if (p.faction === "Ecaz" && p.loyalLeader != null) {
      poc = f(poc + f(TWO_PI / 90));
      const at = pos(b, poc, tr);
      const leader = p.loyalLeader;
      out.push(<MapImage key={`${k}loyal`} x={at.X} y={at.Y} diameter={2 * tr} src={b.personImage(leader)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
        popover={`${b.skin.person(leader)} is Loyal to ${b.skin.faction("Ecaz")}`} popoverAsTooltip />);
    }

    for (const sl of p.skilledLeaders) {
      poc = f(poc + f(TWO_PI / 90));
      const at = pos(b, poc, tr);
      const text = pos(b, poc);
      out.push(<MapImage key={`${k}sl${sl.id}`} x={at.X} y={at.Y} diameter={2 * tr} src={b.personImage(sl.id)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
        popover={() => pop.skilledLeader(b, sl.id, sl.skill)} popoverStyle="" />);
      out.push(<MapText key={`${k}slt${sl.id}`} toShow={b.skin.get("LeaderSkillCardName_STR", sl.skill)} x={text.X} y={text.Y} font={b.s("SKILL_FONT")} fontColor={b.s("SKILL_FONTCOLOR")}
        borderWidth={b.n("SKILL_FONT_BORDERWIDTH")} borderColor={b.s("SKILL_FONT_BORDERCOLOR")} align="center" maxWidth={f(3 * tr)}
        popover={() => pop.skilledLeader(b, sl.id, sl.skill)} popoverStyle="" />);
    }

    if (p.faction === "Ecaz" && p.ambassadors.length > 0) {
      poc = f(poc + f(TWO_PI / 90));
      const at = pos(b, poc, f(0.5 * tr));
      let yOffset = f(f(f(-0.5) * p.ambassadors.length) * f(0.2)) * tr;
      let xOffset = 0;
      p.ambassadors.forEach((a, i) => {
        out.push(<MapImage key={`${k}amb${i}`} x={at.X + xOffset} y={at.Y + yOffset} diameter={tr} src={b.url("AmbassadorImage_URL", a)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
          popover={() => pop.ambassador(b, a)} popoverStyle="" />);
        yOffset += f(f(0.2) * tr);
        xOffset = xOffset === 0 ? f(0.5 * tr) : 0;
      });
    }

    if (p.faction === "Richese" && p.latestRevealedNoField != null) {
      poc = f(poc + f(TWO_PI / 90));
      const at = pos(b, poc, fr);
      const text = pos(b, poc);
      out.push(<MapImage key={`${k}nf`} x={at.X} y={at.Y} diameter={2 * fr} src={b.url("FactionSpecialForceImage_URL", "Richese")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
        popover={`Latest revealed No-Field: ${p.latestRevealedNoField}`} popoverAsTooltip />);
      out.push(<MapText key={`${k}nft`} toShow={String(p.latestRevealedNoField)} x={text.X} y={text.Y} font={b.s("FORCETOKEN_FONT")} fontColor={b.s("FORCETOKEN_FONTCOLOR")}
        borderWidth={b.n("FORCETOKEN_FONT_BORDERWIDTH")} borderColor={b.s("FORCETOKEN_FONT_BORDERCOLOR")} align="center" />);
    }

    const later = p.bribes + p.bankedResources + p.extortion;
    const spiceMarks: [string, number][] = [];
    if (later > 0) spiceMarks.push([`To be collected during ${b.skin.enumValue("MainPhase", "Contemplate")}`, later]);
    if (p.resourcesAfterBidding > 0) spiceMarks.push([`Subbed ${b.skin.concept("Resource")}`, p.resourcesAfterBidding]);
    spiceMarks.forEach(([tooltip, amount], i) => {
      poc = f(poc + f(TWO_PI / 90));
      const at = pos(b, poc, fr);
      const text = pos(b, poc);
      out.push(<MapImage key={`${k}sp${i}`} x={at.X} y={at.Y} diameter={2 * fr} src={b.plainUrl("Harvester_URL")} popover={tooltip} popoverAsTooltip
        backgroundCircleColor="black" backgroundCircleDiameter={f(1.5 * fr)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} />);
      out.push(<MapText key={`${k}spt${i}`} toShow={String(amount)} x={text.X} y={text.Y} font={b.s("RESOURCETOKEN_FONT")} fontColor={b.s("RESOURCETOKEN_FONTCOLOR")}
        popover={tooltip} popoverAsTooltip borderWidth={b.n("RESOURCETOKEN_FONT_BORDERWIDTH")} borderColor={b.s("RESOURCETOKEN_FONT_BORDERCOLOR")} align="center" />);
    });

    reserves.forEach((r, i) => {
      out.push(<MapForce key={`${k}r${i}`} x={r.x} y={r.y} special={r.special} amount={r.amount} faction={r.p.faction} threatened={false} showPopover font={b.s("SKILL_FONT")} diameter={fr} />);
    });

    out.push(<MapText key={`${k}name`} toShow={p.name} x={position.X} y={y} font={b.s("PLAYERNAME_FONT")} fontColor={b.s("PLAYERNAME_FONTCOLOR")}
      borderWidth={b.n("PLAYERNAME_FONT_BORDERWIDTH")} borderColor={b.s("PLAYERNAME_FONT_BORDERCOLOR")} align="center" maxWidth={6 * tr} />);
  }
  return <div>{out}</div>;
}

// Bids.razor.
export function Bids() {
  const b = useBoard();
  const v = b.view;
  if (v.phase !== "Bidding" && v.phase !== "BlackMarketBidding") return null;
  const bids = new Map((v.bidding?.bids ?? []).map((x) => [x.faction, x.bid]));
  return (
    <>
      {v.players.filter((p) => !v.status.waitingFor.includes(p.faction)).map((p) => {
        const position = b.playerTokenPosition(p.seat);
        let text = "";
        let color = "white";
        const bid = bids.get(p.faction);
        if (!p.hasRoomForCards) {
          text = "FULL";
          color = "rgb(200,200,200)";
        } else if (bid) {
          text = bid.t === "pass" ? "PASS" : bid.t === "bid" ? `BID ${bid.amount}` : "READY";
          color = bid.t === "pass" ? "rgb(255,200,200)" : "rgb(200,255,200)";
        }
        return <MapText key={p.faction} toShow={text} x={position.X} y={position.Y + b.tokenRadius} font={b.s("TURN_FONT")} fontColor={color} borderWidth={2} borderColor="black" align="center" />;
      })}
    </>
  );
}
