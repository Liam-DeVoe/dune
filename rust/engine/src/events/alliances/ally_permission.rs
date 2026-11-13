use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct AllyPermission {
    pub ally_may_ship_as_guild: bool,
    pub emperor_will_pay_for_extra_revival: i32,
    pub fremen_will_protect_from_monster: bool,
    pub fremen_allows_three_free_revivals: bool,
    pub fremen_shares_prescience: bool,
    pub fremen_refunds_battle_dial: bool,
    pub ally_may_revive_as_tleilaxu: bool,
    pub ally_may_replace_cards: bool,
    pub atreides_shares_prescience: bool,
    pub bene_gesserit_allows_use_of_voice: bool,
    pub richese_allows_use_of_no_field: bool,
    pub moritani_allows_keeping_cards: bool,
    pub ecaz_shares_ambassadors: bool,
    pub permitted_resources: i32,
    pub permitted_karma_card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for AllyPermission {
    fn default() -> Self {
        AllyPermission {
            ally_may_ship_as_guild: Default::default(),
            emperor_will_pay_for_extra_revival: Default::default(),
            fremen_will_protect_from_monster: Default::default(),
            fremen_allows_three_free_revivals: Default::default(),
            fremen_shares_prescience: Default::default(),
            fremen_refunds_battle_dial: Default::default(),
            ally_may_revive_as_tleilaxu: Default::default(),
            ally_may_replace_cards: Default::default(),
            atreides_shares_prescience: Default::default(),
            bene_gesserit_allows_use_of_voice: Default::default(),
            richese_allows_use_of_no_field: Default::default(),
            moritani_allows_keeping_cards: Default::default(),
            ecaz_shares_ambassadors: Default::default(),
            permitted_resources: Default::default(),
            permitted_karma_card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl AllyPermission {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn guild_allows_shipping_discount(&self) -> bool {
        self.ally_may_ship_as_guild
    }

    pub fn set_guild_allows_shipping_discount(&mut self, value: bool) {
        self.ally_may_ship_as_guild = value;
    }

    pub fn tleilaxu_allows_revival_discount(&self) -> bool {
        self.ally_may_revive_as_tleilaxu
    }

    pub fn set_tleilaxu_allows_revival_discount(&mut self, value: bool) {
        self.ally_may_revive_as_tleilaxu = value;
    }

    pub fn ixian_allows_replacing_cards(&self) -> bool {
        self.ally_may_replace_cards
    }

    pub fn set_ixian_allows_replacing_cards(&mut self, value: bool) {
        self.ally_may_replace_cards = value;
    }

    pub fn permitted_karma_card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::lookup().find(self.permitted_karma_card_id)
    }

    pub fn set_permitted_karma_card(&mut self, value: Option<TreacheryCard>) {
        self.permitted_karma_card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let ally = game.player(self.player()).ally;

        match self.initiator() {
            Faction::Guild => game.guild_allows_shipping_discount = self.guild_allows_shipping_discount(),
            Faction::Tleilaxu => game.tleilaxu_allows_revival_discount = self.tleilaxu_allows_revival_discount(),
            Faction::Ixian => game.ixian_allows_replacing_cards = self.ixian_allows_replacing_cards(),
            Faction::Emperor => game.emperor_will_pay_for_extra_revival = self.emperor_will_pay_for_extra_revival,
            Faction::Fremen => {
                game.fremen_will_protect_from_monster = self.fremen_will_protect_from_monster;
                game.fremen_allows_three_free_revivals = self.fremen_allows_three_free_revivals;
                game.fremen_shares_prescience = self.fremen_shares_prescience;
                game.fremen_refunds_battle_dial = self.fremen_refunds_battle_dial;
            }
            Faction::Atreides => game.atreides_shares_prescience = self.atreides_shares_prescience,
            Faction::BeneGesserit => game.bene_gesserit_allows_use_of_voice = self.bene_gesserit_allows_use_of_voice,
            Faction::Richese => game.richese_allows_use_of_no_field = self.richese_allows_use_of_no_field,
            Faction::Moritani => game.moritani_allows_keeping_cards = self.moritani_allows_keeping_cards,
            Faction::Ecaz => game.ecaz_shares_ambassadors = self.ecaz_shares_ambassadors,
            _ => {}
        }

        // C# parity: with no ally this writes the Faction.None entry (an indexer set, so no throw).
        game.permitted_use_of_ally_spice.insert(ally, self.permitted_resources);
        if let Some(card) = self.permitted_karma_card() {
            game.permitted_use_of_ally_karma.insert(ally, card);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " change ally permissions")
    }

}

impl GameEventMethods for AllyPermission {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AllyPermission
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AllyPermission(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AllyPermission::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AllyPermission::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AllyPermission::get_message(self, game)
    }

}
