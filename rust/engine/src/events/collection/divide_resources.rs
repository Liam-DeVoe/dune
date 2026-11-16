use crate::prelude::*;

/// The first faction of a shared collection proposes how to divide the spice, or passes to split
/// it evenly.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DivideResources {
    pub portion_to_first_player: u16,
    #[serde(skip)]
    pub passed: bool,
}

impl DivideResources {
    pub fn get_resources_to_be_divided(g: &Game) -> Option<ResourcesToBeDivided> {
        g.collection.collected_resources_to_be_divided.first().cloned()
    }

    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        g.turn.current_phase == Phase::DividingCollectedResources
            && Some(Self::get_resources_to_be_divided(g).expect("resources to be divided").first_faction) == g.player(p).faction
    }

    pub fn gained_by_other_faction(tbd: Option<&ResourcesToBeDivided>, agreed: bool, portion_to_first_player: i32) -> i32 {
        // tbd.Amount is read before GainedByFirstFaction runs.
        let amount = tbd.expect("resources to be divided").amount;
        amount - Self::gained_by_first_faction(tbd, agreed, portion_to_first_player)
    }

    pub fn gained_by_first_faction(tbd: Option<&ResourcesToBeDivided>, agreed: bool, portion_to_first_player: i32) -> i32 {
        if agreed {
            portion_to_first_player
        } else {
            half_rounded_toward_zero(tbd.expect("resources to be divided").amount)
        }
    }
}

impl EventRules for DivideResources {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if self.passed {
            return Ok(());
        }

        let to_be_divided = Self::get_resources_to_be_divided(game);
        let Some(to_be_divided) = to_be_divided else { return Err(Rejection(express!("No collections to be divided"))) };
        if i32::from(self.portion_to_first_player) > Self::get_resources_to_be_divided(game).expect("resources to be divided").amount {
            return Err(Rejection(express!("Too much assigned to ", to_be_divided.first_faction)));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.collection.current_division_proposal = Some(DivisionProposal { portion_to_first_player: self.portion_to_first_player });

        if self.passed {
            game.divide_resources_from_collection(false);
            let any = !game.collection.collected_resources_to_be_divided.is_empty();
            game.enter_or_else(any, Phase::DividingCollectedResources, Game::end_collection_main_phase);
        } else {
            let to_be_divided = Self::get_resources_to_be_divided(game);
            let gained_by_other_faction = Self::gained_by_other_faction(to_be_divided.as_ref(), true, i32::from(self.portion_to_first_player));
            let other = to_be_divided.required("to be divided")?.other_faction;
            game.log(parts![by.faction, " propose that they take ", Payment::of(i32::from(self.portion_to_first_player)), " and ", other, " take ", Payment::of(gained_by_other_faction)]);
            game.enter(Phase::AcceptingResourceDivision, true);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        express!("Collected resources were divided")
    }
}
