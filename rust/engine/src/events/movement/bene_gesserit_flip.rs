use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BeneGesseritFlip {
    pub as_advisors: bool,
}

impl BeneGesseritFlip {
    /// The territory of the latest Bene Gesserit intrusion. Panics when there is none.
    pub fn get_territory(g: &Game) -> Territory {
        g.last_bene_gesserit_intrusion().expect("a Bene Gesserit intrusion").territory(g)
    }

    pub fn log_record(&self, game: &Game, by: By) -> BeneGesseritReaction {
        let territory = BeneGesseritFlip::get_territory(game);
        let become_ = (game.player(by.player()).special_forces_in_territory(Some(territory)) > 0) ^ self.as_advisors;
        BeneGesseritReaction::new(
            if become_ { BeneGesseritReactionKind::Become } else { BeneGesseritReactionKind::StayAs },
            territory.into(),
            self.as_advisors,
        )
    }

    pub fn get_dynamic_message(&self, game: &Game, by: By) -> Message {
        let territory = BeneGesseritFlip::get_territory(game);
        let has_advisors_there = game.player(by.player()).special_forces_in_territory(Some(territory)) > 0;

        let force: Part = if self.as_advisors { FactionSpecialForce::BeneGesserit.into() } else { FactionForce::BeneGesserit.into() };
        express!(
            by.faction,
            if has_advisors_there ^ self.as_advisors { " become " } else { " stay as " },
            force,
            " in ",
            territory
        )
    }
}

impl EventRules for BeneGesseritFlip {
    fn validate(&self, _game: &Game, by: By) -> Result<(), Rejection> {
        if !by.is(Faction::BeneGesserit) {
            return Err(Rejection(express!("Your faction can't flip")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if game.last_bene_gesserit_intrusion().is_none() {
            game_error!("There is no Bene Gesserit intrusion");
        }
        let m = self.get_dynamic_message(game, by).with_log(self.log_record(game, by));
        game.log(parts![m]);

        let territory = BeneGesseritFlip::get_territory(game);
        Player::flip_forces_in_territory(game, by.player(), territory, self.as_advisors)?;

        game.flip_bene_gesserit_advisors_when_alone()?;

        game.dequeue_intrusion(IntrusionType::BeneGesseritIntrusion)?;
        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.as_advisors {
            express!(by.faction, " flip to ", FactionSpecialForce::BeneGesserit)
        } else {
            express!(by.faction, " flip to ", FactionForce::BeneGesserit)
        }
    }
}
