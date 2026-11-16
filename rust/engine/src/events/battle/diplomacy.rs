use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Diplomacy {
    pub card: Option<TreacheryCard>,
}

impl Default for Diplomacy {
    fn default() -> Self {
        Diplomacy {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl Diplomacy {
    pub fn valid_cards(g: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        let mut result = Vec::new();
        let plan = g.battle.current_battle.as_ref().expect("current battle").plan_of(g, Some(p)).expect("plan of");
        if let Some(weapon) = plan.weapon() {
            if weapon.is_useless() && plan.defense().is_none() {
                result.push(weapon);
            }
        }
        if let Some(defense) = plan.defense() {
            if defense.is_useless() {
                result.push(defense);
            }
        }
        result
    }

    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        if g.has_skill_in_front_of_shield(p, LeaderSkill::Diplomat) && g.battle.current_diplomacy.is_none() {
            let plan = g.battle.current_battle.as_ref().expect("current battle").plan_of(g, Some(p));
            let plan_of_opponent = g.battle.current_battle.as_ref().expect("current battle").plan_of_opponent(g, Some(p));
            let Some(plan) = plan else { return false };
            // plan.Defense is not { IsDefense: true }
            if plan.defense().is_some_and(|d| d.is_defense()) {
                return false;
            }
            // planOfOpponent.Defense is { IsDefense: true } (planOfOpponent! may be null)
            if !plan_of_opponent.expect("plan of opponent").defense().is_some_and(|d| d.is_defense()) {
                return false;
            }
            return !Self::valid_cards(g, p).is_empty();
        }

        false
    }
}

impl EventRules for Diplomacy {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Game.CurrentBattle?.PlanOfOpponent(Player)?.Defense
        let opponent_defense = game
            .battle.current_battle
            .as_ref()
            .and_then(|b| b.plan_of_opponent(game, by.player))
            .and_then(|plan| plan.defense());
        game.log(parts![by.faction, " use Diplomacy to turn ", self.card, " into a ", opponent_defense]);
        game.battle.current_diplomacy = Some(DiplomacyUsed { submitter: by, card: self.card });
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use Diplomacy to turn ", self.card, " into a copy of the opponent's defense")
    }
}
