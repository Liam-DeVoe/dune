// MapComponents/Phases.razor (the turn and the phases, top right) and CardPiles.razor (the decks
// and discard piles, bottom left and bottom right).
import type React from "react";
import { MainPhaseValue, type MainPhase } from "../../generated/enums";
import { f, useBoard } from "./geometry";
import { MapImage, MapText } from "./primitives";
import * as pop from "./popups";

const TURN_FONT_HEIGHT = 16;
const ACTIVE_FONT_HEIGHT = 16;
const PASSIVE_FONT_HEIGHT = 8;

const PHASES: MainPhase[] = ["Storm", "Blow", "Charity", "Bidding", "Resurrection", "ShipmentAndMove", "Battle", "Collection", "Contemplate"];

export function Phases() {
  const b = useBoard();
  const v = b.view;
  const x = b.mapW - 5;
  const tr = b.tokenRadius;
  const out: React.ReactNode[] = [];
  if (v.turn >= 1) {
    out.push(<MapText key="turn" toShow={`Turn ${v.turn} of ${v.maximumTurns}`} x={x} y={f(0.3) * TURN_FONT_HEIGHT} font={b.s("TURN_FONT")} fontColor={b.s("TURN_FONT_COLOR")}
      borderWidth={b.n("TURN_FONT_BORDERWIDTH")} borderColor={b.s("TURN_FONT_BORDERCOLOR")} popover={() => pop.turn(b)} align="right" />);
  }
  if (MainPhaseValue[v.mainPhase] > MainPhaseValue.Setup) {
    let posY = f(f(0.7) * TURN_FONT_HEIGHT);
    const economics = v.economicsStatus !== "None" && v.economicsStatus !== "RemovedFromGame";
    // Skin.Describe(ChoamEconomicsStatus).
    const economicsText = ({ Cancel: "Cancel", CancelFlipped: "Cancel", Double: "Double", DoubleFlipped: "Double", RemovedFromGame: "Removed from game" } as Record<string, string>)[v.economicsStatus] ?? "None";
    for (const phase of PHASES) {
      const name = b.skin.enumValue("MainPhase", phase);
      if (phase === v.mainPhase) {
        posY += f(f(0.7) * ACTIVE_FONT_HEIGHT);
        out.push(<MapText key={phase} toShow={name} x={x} y={posY} font={b.s("PHASE_ACTIVE_FONT")} fontColor={b.s("PHASE_ACTIVE_FONTCOLOR")}
          borderWidth={b.n("PHASE_ACTIVE_FONT_BORDERWIDTH")} borderColor={b.s("PHASE_FONT_BORDERCOLOR")} align="right" />);
        if (economics && phase === "Charity") {
          const diameter = 2 * tr;
          out.push(<MapImage key="choam" x={x - diameter} y={posY - tr} diameter={diameter} src={b.url("FactionImage_URL", "Choam")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} />);
          out.push(<MapText key="choamText" toShow={economicsText} x={x - tr} y={posY} font={b.s("SKILL_FONT")} fontColor="white" borderWidth={1} borderColor="black" align="center" />);
        }
        posY += f(f(0.6) * ACTIVE_FONT_HEIGHT);
      } else {
        posY += PASSIVE_FONT_HEIGHT;
        out.push(<MapText key={phase} toShow={name} x={x} y={posY} font={b.s("PHASE_FONT")} fontColor={b.s("PHASE_FONTCOLOR")}
          borderWidth={b.n("PHASE_FONT_BORDERWIDTH")} borderColor={b.s("PHASE_FONT_BORDERCOLOR")} align="right" />);
        if (economics && phase === "Charity") {
          out.push(<MapImage key="choam" x={x - tr} y={posY - f(0.5 * tr)} diameter={tr} src={b.url("FactionImage_URL", "Choam")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} />);
          out.push(<MapText key="choamText" toShow={economicsText} x={x - f(0.5 * tr)} y={posY} font={b.s("SKILL_FONT")} fontColor="white" borderWidth={1} borderColor="black" align="center" />);
        }
        posY += f(0.5 * PASSIVE_FONT_HEIGHT);
      }
    }
  }
  return <>{out}</>;
}

// CardPiles.razor.
export function CardPiles({ which }: { which: "spice" | "treachery" }) {
  const b = useBoard();
  const v = b.view;
  const piles = v.piles;
  const { X: cardWidth, Y: cardHeight } = b.point("CardSize");
  const spacing = 1;
  const font = { font: b.s("CARDPILE_FONT"), fontColor: b.s("CARDPILE_FONTCOLOR"), borderWidth: b.n("CARDPILE_FONT_BORDERWIDTH"), borderColor: b.s("CARDPILE_FONT_BORDERCOLOR"), align: "center" as const };
  const out: React.ReactNode[] = [];
  const textY = (y: number) => y + f(cardHeight / f(1.5));
  if (which === "spice") {
    let x = b.point("SpiceDeckLocation").X;
    const y = f(f(1.02) * b.point("SpiceDeckLocation").Y);
    if (piles.resourceDeckCount > 0) {
      const top = piles.resourceDeckTop;
      out.push(<MapImage key="deck" x={x} y={y} width={cardWidth} height={cardHeight} src={top != null ? b.resourceCardImage(top) : b.plainUrl("CardBack_ResourceCard_URL")}
        shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={2} popover={top != null ? () => pop.resourceCard(b, top) : null} popoverStyle="" />);
      out.push(<MapText key="deckCount" toShow={String(piles.resourceDeckCount)} x={x + cardWidth / 2} y={textY(y)} {...font} />);
    }
    x += cardWidth + spacing;
    if (piles.latestSpiceCardA != null) {
      const card = piles.latestSpiceCardA;
      out.push(<MapImage key="a" x={x} y={y} width={cardWidth} height={cardHeight} src={b.resourceCardImage(card)} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
        popover={() => pop.resourceDiscardPile(b, piles.resourceDiscardA, card, "A")} popoverStyle="" />);
    }
    if (b.applicable("IncreasedResourceFlow")) {
      out.push(<MapText key="aText" toShow="A" x={x + cardWidth / 2} y={textY(y)} {...font} />);
      x += cardWidth + spacing;
      if (piles.latestSpiceCardB != null) {
        const card = piles.latestSpiceCardB;
        out.push(<MapImage key="b" x={x} y={y} width={cardWidth} height={cardHeight} src={b.resourceCardImage(card)} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
          popover={() => pop.resourceDiscardPile(b, piles.resourceDiscardB, card, "B")} popoverStyle="" />);
      }
      out.push(<MapText key="bText" toShow="B" x={x + cardWidth / 2} y={textY(y)} {...font} />);
    }
    if (piles.sandTrout != null) {
      const card = piles.sandTrout;
      x += cardWidth + spacing;
      out.push(<MapImage key="trout" x={x} y={y} width={cardWidth} height={cardHeight} src={b.resourceCardImage(card)} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
        popover={() => pop.resourceCard(b, card)} popoverStyle="" />);
    }
  } else {
    let x = b.point("TreacheryDeckLocation").X;
    const y = f(f(1.02) * b.point("TreacheryDeckLocation").Y);
    if (piles.treacheryDeckCount > 0) {
      out.push(<MapImage key="deck" x={x} y={y} width={cardWidth} height={cardHeight} src={b.plainUrl("CardBack_TreacheryCard_URL")} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} />);
    }
    out.push(<MapText key="deckCount" toShow={String(piles.treacheryDeckCount)} x={x + cardWidth / 2} y={textY(y)} {...font} />);
    if (piles.treacheryDiscardCount > 0 && piles.treacheryDiscardTop != null) {
      x += cardWidth + spacing;
      out.push(<MapImage key="discard" x={x} y={y} width={cardWidth} height={cardHeight} src={b.cardImage(piles.treacheryDiscardTop)} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
        popover={() => pop.treacheryDiscardPile(b)} popoverStyle="" />);
      out.push(<MapText key="discardCount" toShow={String(piles.treacheryDiscardCount)} x={x + cardWidth / 2} y={textY(y)} {...font} />);
    }
    if (piles.richeseCacheCount > 0) {
      x = b.point("TreacheryDeckLocation").X - cardWidth - spacing;
      out.push(<MapImage key="cache" x={x} y={y} width={cardWidth} height={cardHeight} src={b.plainUrl("CardBack_TreacheryCard_URL")} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1}
        popover={piles.richeseCache ? () => pop.richeseCache(b) : null} popoverStyle="" />);
      out.push(<MapImage key="cacheFaction" x={x + cardWidth / 3} y={y + cardHeight / 8} diameter={cardWidth / 3} src={b.url("FactionImage_URL", "Richese")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} />);
      out.push(<MapText key="cacheCount" toShow={String(piles.richeseCacheCount)} x={x + cardWidth / 2} y={textY(y)} {...font} />);
    }
  }
  return <>{out}</>;
}
