use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AllianceByAmbassador {
    #[serde(skip)]
    pub passed: bool,
}

impl EventRules for AllianceByAmbassador {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let paused = game.ship_move.paused_ambassador_phase.required("paused ambassador phase")?;
        game.enter(paused, true);

        if !self.passed {
            let activated = game.ship_move.current_ambassador_activated.clone().required("current ambassador activated")?;
            game.make_alliance(by.faction.required("initiator")?, activated.initiator().required("ambassador owner")?);

            if activated.ecaz_give_vidal_to_ally {
                game.take_vidal(by.player(), VidalMoment::EndOfTurn);
            }

            if game.turn.has_acted_or_passed.contains_opt(by.faction)
                && game.turn.has_acted_or_passed.contains_opt(activated.initiator())
            {
                game.check_if_forces_should_be_destroyed_by_ally_presence(by.player())?;
            }
            game.flip_bene_gesserit_advisors_when_alone()?;
        } else {
            let activated = game.ship_move.current_ambassador_activated.clone().required("current ambassador activated")?;
            game.log(parts![by.faction, " don't ally with ", activated.initiator()]);
            if activated.ecaz_take_vidal {
                game.take_vidal(activated.player(), VidalMoment::AfterUsedInBattle);
            }
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, if !self.passed { "" } else { " don't" }, " agree to ally")
    }
}
