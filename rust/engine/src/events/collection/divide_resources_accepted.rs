use crate::prelude::*;

/// The other faction agrees to the proposed division of collected spice, or doesn't.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DivideResourcesAccepted {
    #[serde(skip)]
    pub passed: bool,
}

impl DivideResourcesAccepted {
    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        g.turn.current_phase == Phase::AcceptingResourceDivision
            && Self::get_resources_to_be_divided(g).map(|t| t.other_faction) == g.player(p).faction
    }

    pub fn get_resources_to_be_divided(g: &Game) -> Option<ResourcesToBeDivided> {
        g.collection.collected_resources_to_be_divided.first().cloned()
    }
}

impl EventRules for DivideResourcesAccepted {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, _by: By) -> GameResult {
        game.divide_resources_from_collection(!self.passed);
        let any = !game.collection.collected_resources_to_be_divided.is_empty();
        game.enter_or_else(any, Phase::DividingCollectedResources, Game::end_collection_main_phase);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, if !self.passed { "" } else { " don't" }, " agree with the proposed division")
    }
}
