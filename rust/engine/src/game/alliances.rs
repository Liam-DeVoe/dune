use crate::prelude::*;

/// Alliance offers, card trades between allies, and what each faction allows its ally.
#[derive(Debug)]
pub struct Alliances {
    pub current_alliance_offers: Vec<AllianceOffer>,
    pub last_turn_card_was_traded: Option<Turn>,
    pub permitted_use_of_ally_spice: IndexMap<Faction, i32>,
    /// `Option` because `EmperorBidSupport` assigns its `Amounts`, which JSON can leave null.
    pub permitted_use_of_emperor_spice: Option<IndexMap<Faction, i32>>,
    pub permitted_use_of_ally_karma: IndexMap<Faction, TreacheryCard>,
    pub atreides_shares_prescience: bool,
    pub fremen_will_protect_from_monster: bool,
    pub fremen_allows_three_free_revivals: bool,
    pub fremen_shares_prescience: bool,
    pub fremen_refunds_battle_dial: bool,
    pub emperor_will_pay_for_extra_revival: i32,
    pub guild_allows_shipping_discount: bool,
    pub bene_gesserit_allows_use_of_voice: bool,
    pub ixian_allows_replacing_cards: bool,
    pub tleilaxu_allows_revival_discount: bool,
    pub richese_allows_use_of_no_field: bool,
    pub moritani_allows_keeping_cards: bool,
    pub ecaz_shares_ambassadors: bool,
    /// The shipment permissions the Guild granted, in the order granted. They last until the
    /// Guild lose their high threshold.
    pub shipment_permissions: IndexMap<Faction, ShipmentPermission>,
}

/// An open alliance offer (`AllianceOffered`).
#[derive(Clone, Debug, PartialEq)]
pub struct AllianceOffer {
    pub initiator: Option<Faction>,
    pub player: Option<PlayerId>,
    pub target: Option<Faction>,
}

impl AllianceOffer {
    /// The offering player; panics for an offer without one.
    #[track_caller]
    pub fn player(&self) -> PlayerId {
        self.player.expect("the offer has a player")
    }
}

/// The card one ally offers the other (`CardTraded`), until the other answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardTradeOffer {
    pub initiator: Option<Faction>,
    pub player: Option<PlayerId>,
    pub target: Option<Faction>,
    pub card: Option<TreacheryCard>,
    pub requested_card: Option<TreacheryCard>,
}

impl CardTradeOffer {
    /// The offering player; panics for an offer without one.
    #[track_caller]
    pub fn player(&self) -> PlayerId {
        self.player.expect("the offer has a player")
    }
}

impl Alliances {
    pub fn new() -> Self {
        Alliances {
            current_alliance_offers: Vec::new(),
            last_turn_card_was_traded: None,
            permitted_use_of_ally_spice: Default::default(),
            permitted_use_of_emperor_spice: Some(IndexMap::new()),
            permitted_use_of_ally_karma: Default::default(),
            atreides_shares_prescience: Default::default(),
            fremen_will_protect_from_monster: Default::default(),
            fremen_allows_three_free_revivals: Default::default(),
            fremen_shares_prescience: Default::default(),
            fremen_refunds_battle_dial: Default::default(),
            emperor_will_pay_for_extra_revival: Default::default(),
            guild_allows_shipping_discount: Default::default(),
            bene_gesserit_allows_use_of_voice: Default::default(),
            ixian_allows_replacing_cards: Default::default(),
            tleilaxu_allows_revival_discount: Default::default(),
            richese_allows_use_of_no_field: Default::default(),
            moritani_allows_keeping_cards: Default::default(),
            ecaz_shares_ambassadors: Default::default(),
            shipment_permissions: Default::default(),
        }
    }
}


impl Game {
    pub fn make_alliance(&mut self, a: Faction, b: Faction) {
        let player_a = self.get_player(Some(a));
        let player_b = self.get_player(Some(b));
        self.player_mut(player_a.expect("player a")).ally = Some(b);
        self.player_mut(player_b.expect("player b")).ally = Some(a);
        let player_a = player_a.expect("player a");
        let player_b = player_b.expect("player b");
        self.discard_nexus_card(player_a);
        self.discard_nexus_card(player_b);
        self.log(vec![express!(a, " and ", b, " are now allies")
            .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Made, a, b))))
            .into()]);

        self.set_permissions(a, true);
        self.set_permissions(b, true);

        while self.player(player_a).hand_size_exceeded(self) {
            let card = self.player(player_a).treachery_cards.iter().copied().collect::<Vec<_>>().into_iter().random_item(&mut self.rng);
            self.discard_card_of(Some(player_a), card, true);
        }

        while self.player(player_b).hand_size_exceeded(self) {
            let card = self.player(player_b).treachery_cards.iter().copied().collect::<Vec<_>>().into_iter().random_item(&mut self.rng);
            self.discard_card_of(Some(player_b), card, true);
        }
    }

    pub fn break_all_alliances(&mut self) {
        // ToList: snapshot, then re-check HasAlly per player.
        let players: Vec<PlayerId> =
            self.player_indices().filter(|p| self.player(*p).has_ally()).collect();
        for p in players {
            if self.player(p).has_ally() {
                let faction = self.player(p).faction;
                let ally = self.player(p).ally;
                self.log(vec![express!(faction, " and ", ally, " alliance is broken")
                    .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Ended, faction, ally))))
                    .into()]);
                self.break_alliance(faction.expect("faction"));
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
        self.set_permissions(ally.expect("faction"), false);

        self.alliances.permitted_use_of_ally_spice.shift_remove(&f);
        self.alliances.permitted_use_of_ally_spice.shift_remove_opt(ally);
        self.alliances.permitted_use_of_ally_karma.shift_remove(&f);
        self.alliances.permitted_use_of_ally_karma.shift_remove_opt(ally);

        self.player_mut(initiator).ally = None;
        self.player_mut(current_ally).ally = None;
    }

    pub fn decrease_permitted_use_of_ally_spice(&mut self, f: Faction, amount: i32) {
        if self.alliances.permitted_use_of_ally_spice.contains_key(&f) {
            let new_value = self.alliances.permitted_use_of_ally_spice[&f] - amount;
            self.alliances.permitted_use_of_ally_spice.insert(f, 0.max(new_value));
        }
    }

    pub fn set_permissions(&mut self, f: Faction, permission: bool) {
        match f {
            Faction::Atreides => self.alliances.atreides_shares_prescience = permission,
            Faction::Fremen => {
                self.alliances.fremen_will_protect_from_monster = permission;
                self.alliances.fremen_allows_three_free_revivals = permission;
                self.alliances.fremen_shares_prescience = permission;
                self.alliances.fremen_refunds_battle_dial = permission;
            }
            Faction::Emperor => self.alliances.emperor_will_pay_for_extra_revival = if permission { 3 } else { 0 },
            Faction::Guild => self.alliances.guild_allows_shipping_discount = permission,
            Faction::BeneGesserit => self.alliances.bene_gesserit_allows_use_of_voice = permission,
            Faction::Ixian => self.alliances.ixian_allows_replacing_cards = permission,
            Faction::Tleilaxu => self.alliances.tleilaxu_allows_revival_discount = permission,
            Faction::Richese => self.alliances.richese_allows_use_of_no_field = permission,
            Faction::Moritani => self.alliances.moritani_allows_keeping_cards = permission,
            Faction::Ecaz => self.alliances.ecaz_shares_ambassadors = permission,
            _ => {}
        }
    }

    pub fn are_allies(&self, a: impl Into<Option<Faction>>, b: impl Into<Option<Faction>>) -> bool {
        let a: Option<Faction> = a.into();
        let b: Option<Faction> = b.into();
        // Lifted ==: null == b is false.
        match self.get_player(a) {
            Some(p) => self.player(p).ally == b,
            None => false,
        }
    }

    pub fn resources_your_ally_can_pay(&self, p: PlayerId) -> i32 {
        let Some(value) = self.alliances.permitted_use_of_ally_spice.get_opt(self.player(p).faction).copied() else { return 0 };
        let ally = self.get_player(self.player(p).ally);
        match ally {
            None => 0,
            Some(ally) => value.min(i32::from(self.player(ally).resources)),
        }
    }

    pub fn spice_for_bids_emperor_can_pay(&self, f: Faction) -> i32 {
        let Some(value) = self.alliances.permitted_use_of_emperor_spice.as_ref().expect("permitted use of emperor spice").get(&f).copied() else { return 0 };

        let emperor = self.get_player(Some(Faction::Emperor));
        match emperor {
            None => 0,
            Some(emperor) => value.min(i32::from(self.player(emperor).resources)),
        }
    }

    pub fn get_permitted_use_of_ally_karma(&self, f: impl Into<Option<Faction>>) -> Option<TreacheryCard> {
        let f: Option<Faction> = f.into();
        let mut allies = self.player_indices().filter(|p| self.player(*p).ally == f);
        let ally = allies.next();
        // With f == None this matches every unallied player; two of them is an error the
        // ally permission form and the bots rely on (their view or decision fails).
        assert!(allies.next().is_none(), "more than one player allied to {}", name_or_absent(f));

        let ally = ally?;
        let value = self.alliances.permitted_use_of_ally_karma.get_opt(f).copied()?;
        self.player(ally).has(Some(value)).then_some(value)
    }

    pub fn get_permitted_use_of_ally_resources(&self, f: impl Into<Option<Faction>>) -> i32 {
        let f: Option<Faction> = f.into();
        let ally = match self.get_player(f) {
            Some(p) => self.player(p).allied_player(self),
            None => None,
        };

        let value = self.alliances.permitted_use_of_ally_spice.get_opt(f).copied();
        match (value, ally) {
            (Some(value), Some(ally)) => value.min(i32::from(self.player(ally).resources)),
            _ => 0,
        }
    }
}
