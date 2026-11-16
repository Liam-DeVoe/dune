use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct KarmaMonster {
    pub territory: Option<Territory>,
}

impl Default for KarmaMonster {
    fn default() -> Self {
        KarmaMonster {
            territory: Territory::from_id(0),
        }
    }
}

impl KarmaMonster {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game) -> impl Iterator<Item = Territory> {
        g.board.map.territories(false).filter(move |t| !t.is_protected_from_worm())
    }
}

impl EventRules for KarmaMonster {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        let Some(territory) = self.territory else {
            return Err(Rejection(express!("No territory selected")));
        };
        if territory.is_protected_from_worm() {
            return Err(Rejection(express!("Selected territory is protected")));
        }
        if !KarmaMonster::valid_targets(game).any(|t| t == territory) {
            return Err(Rejection(express!("Invalid territory")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Arguments in C# order: Player, the karma card, then the effect (which reads Territory).
        let me = by.player();
        let card = Karma::valid_karma_cards(game, me).next();
        let effect = vec![Part::from("send "), Part::from(Concept::Monster), Part::from(" to "), Part::from(self.territory)];
        game.play_karma(me, card, effect);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        game.board.number_of_monsters += 1;
        let territory = self.territory;
        game.let_monster_appear(territory, false)?;

        if game.phase() == Phase::BlowReport {
            game.enter_alliance_phase(false);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " send ", Concept::Monster, " to ", self.territory)
    }
}
