// MapComponents/BattlePlans.razor and BattlePlan.razor: the battle wheels on the map. Which plans
// and which of their parts this viewer may see is decided by the view (battle.aggressorPlan,
// defenderPlan); a partial plan is drawn at half opacity.
import type React from "react";
import { Fragment, useState } from "react";
import type { PlanView } from "../../generated/PlanView";
import { f, useBoard } from "./geometry";
import { MapForce, MapImage, MapLeader, MapText } from "./primitives";
import * as pop from "./popups";

export function BattlePlans() {
  const b = useBoard();
  const v = b.view;
  const battle = v.battle;
  if (v.mainPhase !== "Battle" || v.phase === "BattleReport" || !battle) return null;
  const w = b.mapW;
  const h = b.mapH;
  return (
    <>
      {battle.aggressorPlan && <BattlePlan key="a" plan={battle.aggressorPlan} x={f(f(0.1) * w)} y={f(f(0.1) * h)} />}
      {battle.defenderPlan && <BattlePlan key="d" plan={battle.defenderPlan} x={f(f(0.4) * w)} y={f(f(0.5) * h)} />}
    </>
  );
}

// Support.Round: whole numbers as they are, others to one decimal place.
function round(x: number): string {
  return Number.isInteger(x) ? String(x) : String(Math.round(x * 1000) / 1000);
}

function BattlePlan({ plan, x: X, y: Y }: { plan: PlanView; x: number; y: number }) {
  const b = useBoard();
  const [showPlan, setShowPlan] = useState(true);
  const full = plan.full;
  const opacity = full ? 1 : f(0.5);
  const fr = b.forceRadius;
  const rr = b.resourceRadius;
  const tr = b.tokenRadius;
  const heroX = b.n("BattleScreenHeroX");
  const heroY = b.n("BattleScreenHeroY");
  const heroW = b.n("BattleWheelHeroWidth");
  const heroH = b.n("BattleWheelHeroHeight");
  const cardX = b.n("BattleWheelCardX");
  const cardY = b.n("BattleWheelCardY");
  const cardW = b.n("BattleWheelCardWidth");
  const cardH = b.n("BattleWheelCardHeight");
  const screenW = b.n("BattleScreenWidth");
  const wheelBorder = plan.isAggressor ? b.s("WHEEL_FONT_AGGRESSOR_BORDERCOLOR") : b.s("WHEEL_FONT_DEFENDER_BORDERCOLOR");
  const wheelBorderWidth = b.n("WHEEL_FONT_BORDERWIDTH");
  const cardFont = { font: b.s("CARDPILE_FONT"), borderWidth: b.n("CARDPILE_FONT_BORDERWIDTH") };
  const out: React.ReactNode[] = [];

  if (showPlan) {
    out.push(<MapImage key="wheel" x={X} y={Y} diameter={screenW} src={b.plainUrl("BattleScreen_URL")} shadowColor={b.shadow} shadowX={3} shadowY={3} shadowBlur={1} opacity={opacity} />);
    out.push(<MapImage key="faction" x={X} y={Y} diameter={screenW} src={b.factionTableImage(plan.faction)} opacity={f(opacity * f(0.3))} />);
    const forceX = X + b.n("BattleWheelForcesX");
    const forceY = Y + b.n("BattleWheelForcesY");

    if (plan.dial != null) {
      out.push(<MapText key="dial" toShow={round(plan.dial)} x={forceX} y={forceY} font={b.s("WHEEL_FONT")} fontColor={b.s("WHEEL_FONTCOLOR")}
        borderWidth={wheelBorderWidth} borderColor={wheelBorder} align="center" />);
      if (plan.ecazContribution != null) {
        out.push(<MapForce key="ecaz" x={forceX - 3 * fr} y={forceY - fr} diameter={2 * fr} faction="Ecaz" special={false} amount={plan.ecazContribution} />);
      }
      let dialBonusX = forceX + 3 * rr;
      if (plan.ecazKarmaBonus != null) {
        out.push(<MapText key="ecazKarma" toShow={`+${plan.ecazKarmaBonus}`} x={dialBonusX} y={forceY} font={b.s("WHEEL_FONT")} fontColor="green"
          borderWidth={wheelBorderWidth} borderColor={wheelBorder} align="center" />);
        dialBonusX += 2 * rr;
      }
      if (plan.homeworldBonus != null) {
        out.push(<MapImage key="hw" x={dialBonusX - tr} y={forceY - tr} diameter={2 * tr} src={b.url("HomeWorldImage_URL", plan.homeworldBonus.world)}
          shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} />);
        out.push(<MapText key="hwBonus" toShow={`+${plan.homeworldBonus.amount}`} x={dialBonusX} y={forceY} popover="Homeworld Bonus" font={cardFont.font} fontColor="green"
          borderWidth={wheelBorderWidth} borderColor={wheelBorder} align="center" />);
        dialBonusX += 2 * rr;
      }
      if (full && plan.reinforcements) {
        out.push(<MapText key="reinforcements" toShow="+2" x={dialBonusX} y={forceY} popover={b.skin.enumValue("TreacheryCardType", "Reinforcements")} font={cardFont.font} fontColor="green"
          borderWidth={wheelBorderWidth} borderColor={wheelBorder} align="center" />);
      }
    }

    if (full) {
      if (plan.cost != null) {
        out.push(<MapImage key="cost" x={forceX + 5 * rr} y={forceY} diameter={rr * 2} src={b.plainUrl("Harvester_URL")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} />);
        out.push(<MapText key="costText" toShow={String(plan.cost)} x={forceX + 6 * rr} y={forceY + rr} font={b.s("RESOURCETOKEN_FONT")} fontColor={b.s("RESOURCETOKEN_FONTCOLOR")}
          borderWidth={b.n("RESOURCETOKEN_FONT_BORDERWIDTH")} borderColor={b.s("RESOURCETOKEN_FONT_BORDERCOLOR")} align="center" />);
      }
      if (plan.freeResourcesAdvantage) {
        out.push(<MapImage key="adv" x={forceX + f(8 * rr)} y={forceY} width={f(f(0.4) * cardW)} height={f(f(0.4) * cardH)} src={b.url("StrongholdCardImage_URL", "FreeResourcesForBattles")}
          shadowColor={b.shadow} shadowX={3} shadowY={3} shadowBlur={1} popover={() => pop.strongholdAdvantage(b, "FreeResourcesForBattles", plan.faction)} />);
      }
    }

    if (plan.heroShown) {
      const hero = plan.hero;
      if (hero == null) {
        if (!full) {
          out.push(<MapText key="noLeader" toShow="no leader" x={X + heroX + f(f(0.5) * heroW)} y={Y + heroY + f(f(0.5) * heroW)} font={cardFont.font} fontColor="white"
            borderWidth={cardFont.borderWidth} borderColor="black" align="center" />);
        }
      } else if (hero.t === "leader") {
        let skillX = X + heroX + f(f(0.6) * heroW);
        const skillY = Y + heroY - f(f(0.6) * heroH);
        const skillW = f(f(0.4) * cardW);
        const skillH = f(f(0.4) * cardH);
        if (hero.skill != null) {
          const skill = hero.skill;
          out.push(<MapImage key="skill" x={skillX} y={skillY} width={skillW} height={skillH} src={b.url("LeaderSkillCardImage_URL", skill)}
            shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} popover={() => pop.leaderSkill(b, skill)} />);
          skillX += f(f(0.5) * cardW);
        }
        if (hero.passiveSkill != null) {
          const skill = hero.passiveSkill;
          out.push(<MapImage key="passive" x={skillX} y={skillY} width={skillW} height={skillH} src={b.url("LeaderSkillCardImage_URL", skill)}
            shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={0} popover={() => pop.leaderSkill(b, skill)} />);
        }
        out.push(<MapLeader key="leader" src={b.personImage(hero.id)} x={X + heroX} y={Y + heroY} diameter={heroW} popover={() => pop.hero(b, { t: "leader", id: hero.id })} />);
        if (hero.bonus != null) {
          out.push(<MapText key="bonus" toShow={`+${hero.bonus}`} x={X + heroX + f(f(0.9) * heroW)} y={Y + heroY + f(f(0.2) * heroW)} font={cardFont.font} fontColor="green"
            borderWidth={cardFont.borderWidth} borderColor="white" align="right" />);
        }
        if (hero.penalty != null) {
          out.push(<MapText key="penalty" toShow={`-${hero.penalty.amount}`} x={X + heroX + f(f(0.9) * heroW)} y={Y + heroY + f(f(0.75) * heroW)}
            popover={`${b.skin.get("LeaderSkillCardName_STR", hero.penalty.skill)} penalty`} font={cardFont.font} fontColor="red"
            borderWidth={cardFont.borderWidth} borderColor="white" align="right" />);
        }
      } else {
        const card = hero.id;
        out.push(<MapImage key="cheapHero" x={X + heroX} y={Y + heroY} width={cardW} height={cardH} src={b.cardImage(card)} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1}
          popover={() => pop.treacheryCard(b, card)} popoverStyle="" />);
      }
      const markY = Y + heroY + f(heroW / f(1.5));
      if (plan.traitorCalled) {
        out.push(<MapText key="traitor" toShow="TRAITOR!" x={X + heroX + f(f(0.5) * heroW)} y={markY} font={cardFont.font} fontColor="red" borderWidth={cardFont.borderWidth} borderColor="white" align="center" />);
      }
      if (plan.facedanced) {
        out.push(<MapText key="facedancer" toShow="FACE DANCER!" x={X + heroX + f(f(0.5) * heroW)} y={markY} font={cardFont.font} fontColor="red" borderWidth={cardFont.borderWidth} borderColor="white" align="center" />);
      }
      if (plan.messiah) {
        out.push(<MapImage key="messiah" x={X + heroX + f(heroW / f(1.5))} y={Y + f(f(0.8) * heroY)} diameter={f(f(0.5) * heroW)} src={b.plainUrl("Messiah_URL")}
          shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} popover={b.skin.concept("Messiah")} />);
      }
    }

    if (plan.defenseShown) {
      if (plan.defense != null) {
        const card = plan.defense;
        out.push(<MapImage key="defense" x={X + cardX + f(f(1.1) * cardW)} y={Y + cardY} width={cardW} height={cardH} src={b.cardImage(card)}
          shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} popover={() => pop.treacheryCard(b, card)} popoverStyle="" />);
      } else if (!full) {
        out.push(<MapText key="noDefense" toShow="no defense" x={X + cardX + f(f(1.6) * cardW)} y={Y + f(f(1.1) * cardY) + f(f(0.5) * cardH)} font={b.s("TURN_FONT")} fontColor="white"
          borderWidth={b.n("TURN_FONT_BORDERWIDTH")} borderColor="black" align="center" />);
      }
    }

    if (plan.weaponShown) {
      if (plan.weapon != null) {
        const card = plan.weapon;
        out.push(<MapImage key="weapon" x={X + cardX} y={Y + cardY} width={cardW} height={cardH} src={b.cardImage(card)}
          shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} popover={() => pop.treacheryCard(b, card)} popoverStyle="" />);
      } else if (!full) {
        out.push(<MapText key="noWeapon" toShow="no weapon" x={X + cardX + f(f(0.5) * cardW)} y={Y + f(f(1.1) * cardY) + f(f(0.5) * cardH)} font={b.s("TURN_FONT")} fontColor="white"
          borderWidth={b.n("TURN_FONT_BORDERWIDTH")} borderColor="black" align="center" />);
      }
    }
  } else {
    out.push(<MapImage key="closed" x={X + f(f(0.5) * screenW) - fr} y={Y} diameter={2 * fr} src={b.plainUrl("BattleScreen_URL")} shadowColor={b.shadow} shadowX={1} shadowY={1} shadowBlur={1} />);
  }

  out.push(<MapImage key="eye" x={X + b.n("BattleWheelForcesX") - f(f(0.5) * fr)} y={Y + f(f(0.25) * fr)} diameter={fr} src={b.plainUrl(showPlan ? "EyeSlash_URL" : "Eye_URL")}
    onClick={() => setShowPlan(!showPlan)} shadowColor="black" shadowX={1} shadowY={1} shadowBlur={0} popover={showPlan ? "Hide Wheel" : "Show Wheel"} popoverAsTooltip />);

  return <Fragment>{out}</Fragment>;
}
