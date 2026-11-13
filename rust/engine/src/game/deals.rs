use crate::prelude::*;

impl Game {
    pub fn record_deal_response(&mut self, offer: &Rc<DealOffered>, faction: Faction, accepted: bool) {
        let key = RcKey(offer.clone());
        if !self.deal_responses.contains_key(&key) {
            // Never a duplicate: the key was just checked.
            self.deal_responses.add_with(key.clone(), NetDictionary::new(), |_| "Treachery.Shared.DealOffered".to_string());
        }

        self.deal_responses.get_mut(&key).nre().insert(faction, accepted);
    }

    pub fn deal_response(&self, offer: &Rc<DealOffered>, faction: Faction) -> Option<bool> {
        let key = RcKey(offer.clone());
        match self.deal_responses.get(&key) {
            Some(responses) => responses.get(&faction).copied(),
            None => None,
        }
    }

    pub fn has_responded_to_deal(&self, offer: &Rc<DealOffered>, faction: Faction) -> bool {
        self.deal_response(offer, faction).is_some()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn deal_recipients(&self, offer: &DealOffered) -> impl Iterator<Item = Faction> {
        let initiator = offer.initiator();
        self.players
            .iter()
            .filter(move |p| {
                // offer.To is an array; a null To throws on .Length.
                p.faction != initiator && p.ally != initiator && {
                    let to = offer.to.as_ref().nre();
                    to.len() == 0 || to.contains(&p.faction)
                }
            })
            .map(move |p| p.faction)
    }

    pub fn start_deal(&mut self, deal: &Deal) {
        self.deals.push(deal.clone());
    }

    pub fn has_deal(&self, f: Faction, type_: DealType) -> bool {
        self.deals.iter().any(|deal| deal.consuming_faction == f && deal.type_ == type_)
    }

    pub fn remove_ended_deals(&mut self, phase: Phase) {
        // Deals are distinct objects, so List.Remove(deal) on the snapshot removes exactly
        // the deals whose End matches.
        self.deals.retain(|deal| deal.end != phase);
    }

    pub fn is_ghola(&self, l: Hero) -> bool {
        let tleilaxu = self.get_player(Some(Faction::Tleilaxu));
        l.faction() != Faction::Tleilaxu
            && tleilaxu.is_some()
            && match l.as_leader() {
                // Leaders.Contains(IHero): reference equality, so only a Leader can match.
                Some(leader) => self.player(tleilaxu.nre()).leaders.contains(&leader),
                None => false,
            }
    }
}

#[derive(Clone, Debug, Canon)]
pub struct Deal {
    pub bound_faction: Faction,
    pub consuming_faction: Faction,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub benefit: i32,
    pub end: Phase,
}

impl Default for Deal {
    fn default() -> Self {
        Deal {
            bound_faction: Default::default(),
            consuming_faction: Default::default(),
            type_: Default::default(),
            deal_parameter1: Default::default(),
            deal_parameter2: Default::default(),
            text: Default::default(),
            benefit: Default::default(),
            end: Default::default(),
        }
    }
}

impl Deal {
    pub fn new() -> Self {
        Self::default()
    }

    /// The only `T` used is `Territory` (bots) and `object` (which holds a
    /// `Territory` or null), so the generic parameter is fixed to `Territory`.
    pub fn get_parameter1(&self, g: &Game) -> Option<Territory> {
        match &self.deal_parameter1 {
            None => None,
            Some(p) => Deal::get_parameter1_game_deal_type_string(g, self.type_, p),
        }
    }

    pub fn get_parameter1_game_deal_type_string(g: &Game, type_: DealType, parameter: &str) -> Option<Territory> {
        match type_ {
            DealType::DontShipOrMoveTo => g.map.territory_lookup().find(dotnet_compat::parse::int_parse(Some(parameter))),
            _ => None,
        }
    }

    pub fn deal_contents_description(&self, g: &Game) -> Message {
        Deal::deal_contents_description_game_deal_type_string_int32_phase_string(
            g,
            self.type_,
            Some(self.text.as_deref().unwrap_or("")),
            self.benefit,
            self.end,
            Some(self.deal_parameter1.as_deref().unwrap_or("")),
        )
    }

    pub fn deal_contents_description_game_deal_type_string_int32_phase_string(g: &Game, type_: DealType, text: Option<&str>, benefit: i32, end: Phase, parameter1: Option<&str>) -> Message {
        // string.IsNullOrWhiteSpace: char.IsWhiteSpace and Rust's char::is_whitespace agree.
        let is_null_or_white_space = text.map_or(true, |t| t.chars().all(char::is_whitespace));
        if !is_null_or_white_space {
            return express!(
                express_part_if!(benefit > 0; "Receive ", Payment::of(benefit), " and "),
                text.nre().to_string(),
                " until ",
                end
            );
        }
        express!(
            express_part_if!(benefit > 0; "Receive ", Payment::of(benefit), " and "),
            Deal::express(type_, Deal::get_parameter1_game_deal_type_string(g, type_, parameter1.unwrap_or("?")).into()),
            " until ",
            end
        )
    }

    pub fn express(d: DealType, parameter: ExpressionElement) -> Message {
        let p: ExpressionElement = match parameter {
            ExpressionElement::Null => "...".into(),
            other => other,
        };

        match d {
            DealType::None => express!("Custom deal"),
            DealType::DontShipOrMoveTo => express!("Don't ship or move to ", p),
            DealType::ShareBiddingPrescience => express!("Share treachery card prescience"),
            DealType::ShareResourceDeckPrescience => express!("Share prescience of the top ", Concept::Resource, "card"),
            DealType::ShareStormPrescience => express!("Share storm prescience"),
            DealType::ForfeitBattle => express!("Forfeit this battle (no weapons and defenses, lowest leader, zero dial)"),
            DealType::TellDiscardedTraitors => express!("Tell which traitors were discarded"),
            _ => express!("unknown deal type"),
        }
    }
}
