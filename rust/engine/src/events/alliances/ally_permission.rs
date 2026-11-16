use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct AllyPermission {
    pub ally_may_ship_as_guild: bool,
    pub emperor_will_pay_for_extra_revival: u16,
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
    pub permitted_resources: u16,
    pub permitted_karma_card: Option<TreacheryCard>,
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
            permitted_karma_card: TreacheryCardManager::get(0),
        }
    }
}

impl AllyPermission {
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
}

impl EventRules for AllyPermission {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let ally = game.player(by.player()).ally;

        match by.faction {
            Some(Faction::Guild) => game.alliances.guild_allows_shipping_discount = self.guild_allows_shipping_discount(),
            Some(Faction::Tleilaxu) => game.alliances.tleilaxu_allows_revival_discount = self.tleilaxu_allows_revival_discount(),
            Some(Faction::Ixian) => game.alliances.ixian_allows_replacing_cards = self.ixian_allows_replacing_cards(),
            Some(Faction::Emperor) => game.alliances.emperor_will_pay_for_extra_revival = i32::from(self.emperor_will_pay_for_extra_revival),
            Some(Faction::Fremen) => {
                game.alliances.fremen_will_protect_from_monster = self.fremen_will_protect_from_monster;
                game.alliances.fremen_allows_three_free_revivals = self.fremen_allows_three_free_revivals;
                game.alliances.fremen_shares_prescience = self.fremen_shares_prescience;
                game.alliances.fremen_refunds_battle_dial = self.fremen_refunds_battle_dial;
            }
            Some(Faction::Atreides) => game.alliances.atreides_shares_prescience = self.atreides_shares_prescience,
            Some(Faction::BeneGesserit) => game.alliances.bene_gesserit_allows_use_of_voice = self.bene_gesserit_allows_use_of_voice,
            Some(Faction::Richese) => game.alliances.richese_allows_use_of_no_field = self.richese_allows_use_of_no_field,
            Some(Faction::Moritani) => game.alliances.moritani_allows_keeping_cards = self.moritani_allows_keeping_cards,
            Some(Faction::Ecaz) => game.alliances.ecaz_shares_ambassadors = self.ecaz_shares_ambassadors,
            _ => {}
        }

        if ally != None {
            game.alliances.permitted_use_of_ally_spice.insert(ally.required("faction")?, i32::from(self.permitted_resources));
            if let Some(card) = self.permitted_karma_card {
                game.alliances.permitted_use_of_ally_karma.insert(ally.required("faction")?, card);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " change ally permissions")
    }
}
