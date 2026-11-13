use crate::prelude::*;

use crate::compat::random_ext::RandomExt;

impl Game {
    pub fn make_alliance(&mut self, a: Faction, b: Faction) {
        let player_a = self.get_player(Some(a));
        let player_b = self.get_player(Some(b));
        self.player_mut(player_a.nre()).ally = b;
        self.player_mut(player_b.nre()).ally = a;
        let player_a = player_a.nre();
        let player_b = player_b.nre();
        self.discard_nexus_card(player_a);
        self.discard_nexus_card(player_b);
        self.log(vec![express!(a, " and ", b, " are now allies")
            .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Made, a, b))))
            .into()]);

        self.set_permissions(a, true);
        self.set_permissions(b, true);

        while self.player(player_a).hand_size_exceeded(self) {
            let card = self.player(player_a).treachery_cards.iter().copied().random_or_default(self.random.as_ref().nre());
            self.discard_player_treachery_card_boolean(Some(player_a), card, true);
        }

        while self.player(player_b).hand_size_exceeded(self) {
            let card = self.player(player_b).treachery_cards.iter().copied().random_or_default(self.random.as_ref().nre());
            self.discard_player_treachery_card_boolean(Some(player_b), card, true);
        }
    }

    pub fn break_all_alliances(&mut self) {
        // ToList: snapshot, then re-check HasAlly per player.
        let players: Vec<PlayerIndex> =
            self.player_indices().filter(|p| self.player(*p).has_ally()).collect();
        for p in players {
            if self.player(p).has_ally() {
                let faction = self.player(p).faction;
                let ally = self.player(p).ally;
                self.log(vec![express!(faction, " and ", ally, " alliance is broken")
                    .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Ended, faction, ally))))
                    .into()]);
                self.break_alliance(faction);
            }
        }
    }

    pub fn break_alliance(&mut self, f: Faction) {
        let initiator = self.get_player(Some(f));
        let current_ally = match initiator {
            Some(i) => self.player(i).allied_player(self),
            None => None,
        };

        let (Some(initiator), Some(current_ally)) = (initiator, current_ally) else { return };

        self.set_permissions(f, false);
        let ally = self.player(initiator).ally;
        self.set_permissions(ally, false);

        self.permitted_use_of_ally_spice.shift_remove(&f);
        self.permitted_use_of_ally_spice.shift_remove(&ally);
        self.permitted_use_of_ally_karma.shift_remove(&f);
        self.permitted_use_of_ally_karma.shift_remove(&ally);

        self.player_mut(initiator).ally = Faction::None;
        self.player_mut(current_ally).ally = Faction::None;
    }

    pub fn decrease_permitted_use_of_ally_spice(&mut self, f: Faction, amount: i32) {
        if self.permitted_use_of_ally_spice.contains_key(&f) {
            let new_value = self.permitted_use_of_ally_spice[&f] - amount;
            self.permitted_use_of_ally_spice.insert(f, 0.max(new_value));
        }
    }

    pub fn set_permissions(&mut self, f: Faction, permission: bool) {
        match f {
            Faction::Atreides => self.atreides_shares_prescience = permission,
            Faction::Fremen => {
                self.fremen_will_protect_from_monster = permission;
                self.fremen_allows_three_free_revivals = permission;
                self.fremen_shares_prescience = permission;
                self.fremen_refunds_battle_dial = permission;
            }
            Faction::Emperor => self.emperor_will_pay_for_extra_revival = if permission { 3 } else { 0 },
            Faction::Guild => self.guild_allows_shipping_discount = permission,
            Faction::BeneGesserit => self.bene_gesserit_allows_use_of_voice = permission,
            Faction::Ixian => self.ixian_allows_replacing_cards = permission,
            Faction::Tleilaxu => self.tleilaxu_allows_revival_discount = permission,
            Faction::Richese => self.richese_allows_use_of_no_field = permission,
            Faction::Moritani => self.moritani_allows_keeping_cards = permission,
            Faction::Ecaz => self.ecaz_shares_ambassadors = permission,
            _ => {}
        }
    }

    pub fn are_allies(&self, a: Faction, b: Faction) -> bool {
        // Lifted ==: null == b is false.
        match self.get_player(Some(a)) {
            Some(p) => self.player(p).ally == b,
            None => false,
        }
    }

    pub fn resources_your_ally_can_pay(&self, p: PlayerIndex) -> i32 {
        let Some(value) = self.permitted_use_of_ally_spice.get(&self.player(p).faction).copied() else { return 0 };
        let ally = self.get_player(Some(self.player(p).ally));
        match ally {
            None => 0,
            Some(ally) => value.min(self.player(ally).resources),
        }
    }

    pub fn spice_for_bids_emperor_can_pay(&self, f: Faction) -> i32 {
        let Some(value) = self.permitted_use_of_emperor_spice.as_ref().nre().get(&f).copied() else { return 0 };

        let emperor = self.get_player(Some(Faction::Emperor));
        match emperor {
            None => 0,
            Some(emperor) => value.min(self.player(emperor).resources),
        }
    }

    pub fn get_permitted_use_of_ally_karma(&self, f: Faction) -> Option<TreacheryCard> {
        let mut allies = self.player_indices().filter(|p| self.player(*p).ally == f);
        let ally = allies.next();
        // With f == Faction::None this matches every unallied player; two of them is an error the
        // ally permission form and the bots rely on (their view or decision fails).
        assert!(allies.next().is_none(), "more than one player allied to {f}");

        let ally = ally?;
        let value = self.permitted_use_of_ally_karma.get(&f).copied()?;
        self.player(ally).has(Some(value)).then_some(value)
    }

    pub fn get_permitted_use_of_ally_resources(&self, f: Faction) -> i32 {
        let ally = match self.get_player(Some(f)) {
            Some(p) => self.player(p).allied_player(self),
            None => None,
        };

        let value = self.permitted_use_of_ally_spice.get(&f).copied();
        match (value, ally) {
            (Some(value), Some(ally)) => value.min(self.player(ally).resources),
            _ => 0,
        }
    }
}
