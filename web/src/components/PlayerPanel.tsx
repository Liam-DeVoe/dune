// OtherComponents/PlayerPanel.razor for the viewer's own faction: spice and reserves in a bar, the
// leaders with the traitors (or face dancers) in a recessed well at the right, any faction
// extras, and the hand of cards under a rule. Styles in _player-panel.scss.
//
// The C# panel can also show another player's panel under a collapsible header, for a seated
// viewer without a faction (GameColumns.razor). Factions are assigned before TradingFactions
// ends, so that never happens in a game this client shows; the view has no other player's
// shield to draw.
import { Fragment, type CSSProperties } from "react";
import type { Faction, LeaderSkill, TerrorType } from "../generated/enums";
import { PhaseValue } from "../generated/enums";
import type { HeroCard } from "../generated/HeroCard";
import type { Part } from "../generated/Part";
import type { PrivatePlayer } from "../generated/PrivatePlayer";
import type { PublicPlayer } from "../generated/PublicPlayer";
import { ambassadorPopup, leaderPopup, messiahPopup, treacheryCardPopup } from "../popups";
import type { Skin } from "../skin";
import { useGame } from "../state/context";
import { FactionChip } from "./FactionChip";
import { FactionComponent } from "./FactionComponent";
import { ForceToken } from "./ForceToken";
import { Image } from "./Image";
import { useMenu } from "./menuState";
import { RichText } from "./RichText";
import { SimpleNumber } from "./SimpleNumber";
import { SpiceLine } from "./SpiceLine";

export function PlayerPanel() {
  const { view } = useGame();
  const me = view.me;
  const pub = view.players.find((p) => p.faction === me?.faction);
  if (!me || !pub || PhaseValue[view.phase] < PhaseValue.TradingFactions) return null;
  return (
    <div className="card p-1 mb-1" style={{ animation: "fadeIn 3s" }}>
      <div className="card-body p-0 bg-transparent player-panel">
        <div className="card-text h-100">
          {PhaseValue[view.phase] >= PhaseValue.HarkonnenMulligan
            ? <Shield me={me} pub={pub} />
            : <FactionComponent faction={me.faction} />}
        </div>
      </div>
    </div>
  );
}

function Shield({ me, pub }: { me: PrivatePlayer; pub: PublicPlayer }) {
  const { view, skin } = useGame();
  const menu = useMenu();
  const faction = me.faction;
  const afterPrediction = PhaseValue[view.phase] > PhaseValue.BeneGesseritPredicting;
  const alive = me.leaders.filter((l) => l.alive);
  const dead = me.leaders.filter((l) => !l.alive);
  return (
    <>
      <div className="pp-bar">
        <FactionChip faction={faction} />
        <span className="pp-stat"><SpiceLine text={resourcesText(skin, me, pub)} size={24} /></span>
        <span className="pp-stat">
          <ForceToken faction={faction} size={22} imgClass="m-0 p-0" title={skin.get("ForceName_STR", faction)} />{pub.forcesInReserve}
        </span>
        {pub.hasSpecialForces && (
          <span className="pp-stat">
            <ForceToken faction={faction} special size={22} imgClass="m-0 p-0" title={skin.get("SpecialForceName_STR", faction)} />{pub.specialForcesInReserve}
          </span>
        )}
      </div>

      {/* Leaders, then the traitors held (or face dancers) in a recessed well at the right. */}
      <div className="pp-people">
        {alive.map((l) => (
          <Image key={l.id} src={skin.personImage(l.id)} alt="" highlighted={l.skilled} width={44} noShadow popover={leaderPopup(skin, l.id, l.skill)} />
        ))}
        {dead.map((l) => (
          <Image key={l.id} src={skin.personImage(l.id)} alt="" darkened width={44} noShadow popover={leaderPopup(skin, l.id, l.skill)} />
        ))}
        {me.faceDancers.length > 0
          ? <Well title="Face Dancers" heroes={me.faceDancers} />
          : me.traitors.length > 0 && <Well title={me.traitors.length === 1 ? "Your traitor" : "Your traitors"} heroes={me.traitors} />}
      </div>

      <div className="pp-extras">
        {me.nextStormMoves !== null && (
          <div>The next storm will move <SimpleNumber>{me.nextStormMoves}</SimpleNumber> sectors.</div>
        )}
        {faction === "BeneGesserit" && afterPrediction && me.predictedFaction !== null && (
          <div>Prediction: <FactionChip faction={me.predictedFaction} /> <SimpleNumber size={30}>{me.predictedTurn}</SimpleNumber></div>
        )}
        {me.messiah !== null && <Messiah forcesKilled={me.messiah.forcesKilledInBattle} available={me.messiah.available} />}
        {faction === "Ecaz" && afterPrediction && (
          <div>
            {" Ambassadors: "}
            {pub.ambassadors.map((a, i) => (
              <Image key={i} src={skin.url(skin.get("AmbassadorImage_URL", a))} alt={skin.get("AmbassadorName_STR", a)} width={30} popover={ambassadorPopup(skin, a)} />
            ))}
          </div>
        )}
        {faction === "Moritani" && afterPrediction && me.unplacedTerror !== null && (
          <div>
            {" Available Terror: "}
            <RichText parts={terrorParts(skin, me.unplacedTerror)} size={30} textBorder={moritaniBorder} />
          </div>
        )}
        {view.mainPhase === "ShipmentAndMove" && view.piles.resourceDeckTop !== null && (
          <div>Top card of {skin.concept("Resource")} Deck: <RichText parts={[resourceCardPart(skin, view.piles.resourceDeckTop)]} /></div>
        )}
      </div>

      {/* The hand, under a rule. It keeps its height when empty. A card you can play now glows;
          clicking it opens its panel (CardPlayPopover). */}
      <div className="pp-cards">
        {me.hand.map((c, i) => {
          const card = <Image src={skin.cardImage(c.id)} alt={skin.card(c.id)} width={48} noShadow popover={treacheryCardPopup(skin, c.id)} />;
          return c.playableWith.length > 0
            ? (
              <span key={i} className={`pp-card playable ${menu.cardPlay === c.id ? "open" : ""}`}
                    onClick={(e) => menu.toggleCardPlay(c.id, e.clientY)}>
                {card}
              </span>
            )
            : <Fragment key={i}>{card}</Fragment>;
        })}
        {me.cardSetAside !== null && (
          <Image src={skin.cardImage(me.cardSetAside)} alt={skin.card(me.cardSetAside)} width={48} noShadow popover={treacheryCardPopup(skin, me.cardSetAside)} />
        )}
      </div>

      {me.nexus !== null && (
        <div className="pp-extras">Nexus: <Image width={30} alt="" src={skin.url(skin.get("FactionFacedownImage_URL", me.nexus))}
          popover={`<div class='pop-art'><img src='${skin.get("NexusCardImage_URL", me.nexus)}'/></div>`} /></div>
      )}
    </>
  );
}

// The traitors or face dancers, each tagged with its faction for the faction chips' hover.
function Well({ title, heroes }: { title: string; heroes: HeroCard[] }) {
  const { skin } = useGame();
  return (
    <span className="pp-well" title={title}>
      {heroes.map((h, i) => {
        const { src, faction, popover } = hero(skin, h);
        return (
          <span key={i} data-faction-of={faction}>
            <Image src={src} alt="" blurred={h.revealed} width={38} noShadow popover={popover} />
          </span>
        );
      })}
    </span>
  );
}

// An IHero's image, faction and popover (Skin.GetPopup(IHero, Game)).
function hero(skin: Skin, h: HeroCard): { src: string; faction: Faction; popover: string } {
  switch (h.hero.t) {
    case "leader":
      return { src: skin.personImage(h.hero.id), faction: skin.leaderFaction(h.hero.id), popover: leaderPopup(skin, h.hero.id, h.skill as LeaderSkill | null) };
    case "messiah":
      return { src: skin.url(skin.str("Messiah_URL")), faction: "Atreides", popover: messiahPopup(skin) };
    case "treacheryCard":
      return { src: skin.cardImage(h.hero.id), faction: "None", popover: treacheryCardPopup(skin, h.hero.id) };
  }
}

function Messiah({ forcesKilled, available }: { forcesKilled: number; available: boolean }) {
  const { skin } = useGame();
  const name = skin.concept("Messiah");
  return (
    <div>
      <img className="img-fluid m-0 p-0" width="32" height="32" src={skin.url(skin.str("Messiah_URL"))} title={name} alt={name} />
      {forcesKilled >= 7
        ? (available ? <span> is available.</span> : <span>has been killed.</span>)
        : <span> will be available after <SimpleNumber>{7 - forcesKilled}</SimpleNumber> more forces are killed in battle.</span>}
    </div>
  );
}

// PlayerPanel.ResourcesText: spice now, then what comes later (bribes and extortion at the end of
// the turn, spice after bidding, banked spice at Mentat Pause), through Skin.Format.
function resourcesText(skin: Skin, me: PrivatePlayer, p: PublicPlayer): string {
  const phase = (ph: string) => skin.enumValue("MainPhase", ph);
  const later = p.bribes + p.extortion;
  const r = me.resources;
  if (p.bankedResources > 0) {
    if (later > 0) {
      if (p.resourcesAfterBidding > 0) {
        return `${r}, ${later} at end of turn, ${p.resourcesAfterBidding} at end of bidding, ${p.bankedResources} at ${phase("Contemplate")}`;
      }
      return `${r}, ${later} at ${phase("Contemplate")}, ${p.bankedResources} at ${phase("Contemplate")}`;
    }
    if (p.resourcesAfterBidding > 0) {
      return `${r}, ${p.resourcesAfterBidding} at end of ${phase("Bidding")}, ${p.bankedResources} at ${phase("Contemplate")}`;
    }
    return `${r}, ${p.bankedResources} at ${phase("Contemplate")}`;
  }
  if (later > 0) {
    if (p.resourcesAfterBidding > 0) return `${r}, ${later} at end of turn, ${p.resourcesAfterBidding} at end of bidding`;
    return `${r}, ${later} at ${phase("Contemplate")}`;
  }
  if (p.resourcesAfterBidding > 0) return `${r}, ${p.resourcesAfterBidding} at end of ${phase("Bidding")}`;
  return `${r}`;
}

// Moritani's colour as the text border of its terror tokens' names.
const moritaniBorder: CSSProperties = {
  color: "var(--parchment)",
  WebkitTextStroke: "0.3px var(--f-moritani)",
  textShadow: "0 0 2px var(--f-moritani)",
};

// Expression.Express(Game.UnplacedTerrorTokens) as ExpressionComponent draws it: the names,
// separated by ", ".
function terrorParts(skin: Skin, tokens: string[]): Part[] {
  return tokens.flatMap((t, i): Part[] => {
    const part: Part = { t: "enum", ty: "TerrorType", v: t as TerrorType, label: skin.get("TerrorTokenName_STR", t) };
    return i === 0 ? [part] : [{ t: "text", v: ", " }, part];
  });
}

// A resource card as rich text, named as Skin.Describe(ResourceCard) names it.
function resourceCardPart(skin: Skin, skinId: number): Part {
  const c = skin.resourceCardInfo(skinId);
  const label = !c ? "?"
    : c.isShaiHulud ? skin.concept("Monster")
    : c.isGreatMaker ? skin.concept("GreatMonster")
    : c.isSandTrout ? skin.concept("BabyMonster")
    : c.location !== null ? skin.territory(skin.locationInfo(c.location)?.territory ?? -1) : "?";
  return { t: "resourceCard", id: skinId, label };
}
