use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct HMSAdvantageChosen {
    pub advantage: Option<StrongholdAdvantage>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for HMSAdvantageChosen {
    fn default() -> Self {
        HMSAdvantageChosen {
            advantage: Default::default(),
            base: Default::default(),
        }
    }
}

impl HMSAdvantageChosen {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        let hms = g.board.map.hidden_mobile_stronghold();
        g.battle.chosen_hms_advantage == Some(None)
            && g.battle.current_battle.is_some()
            && g.battle.current_battle.as_ref().expect("current battle").is_aggressor_or_defender(g, p)
            && g.battle.current_battle.as_ref().expect("current battle").territory == Some(hms.location().territory())
            && g.owns_stronghold(g.player(p).faction, hms.location())
            && !HMSAdvantageChosen::valid_advantages(g, p).is_empty()
    }

    pub fn valid_advantages(g: &Game, p: PlayerIndex) -> Vec<StrongholdAdvantage> {
        let mut result = Vec::new();
        if g.owner_of_location(g.board.map.arrakeen()) == Some(p) {
            result.push(StrongholdAdvantage::FreeResourcesForBattles);
        }
        if g.owner_of_location(g.board.map.carthag()) == Some(p) {
            result.push(StrongholdAdvantage::CountDefensesAsAntidote);
        }
        if g.owner_of_location(g.board.map.sietch_tabr()) == Some(p) {
            result.push(StrongholdAdvantage::CollectResourcesForDial);
        }
        if g.owner_of_location(g.board.map.tueks_sietch()) == Some(p) {
            result.push(StrongholdAdvantage::CollectResourcesForUseless);
        }
        if g.owner_of_location(g.board.map.habbanya_sietch()) == Some(p) {
            result.push(StrongholdAdvantage::WinTies);
        }
        result
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.battle.chosen_hms_advantage = Some(self.advantage);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use the ", self.advantage, " stronghold advantage for this battle")
    }

}

impl GameEventMethods for HMSAdvantageChosen {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::HMSAdvantageChosen
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::HMSAdvantageChosen(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        HMSAdvantageChosen::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        HMSAdvantageChosen::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        HMSAdvantageChosen::get_message(self, game)
    }

}
