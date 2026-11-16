use crate::prelude::*;
use crate::messages::part::Part;

/// Deals offered, the responses to them, and the deals in force.
pub struct Deals {
    pub offers: Vec<Rc<DealOffered>>,
    pub responses: IndexMap<RcKey<DealOffered>, IndexMap<Faction, bool>>,
    pub agreed: Vec<Deal>,
}

impl Deals {
    pub fn new() -> Self {
        Deals {
            offers: Vec::new(),
            responses: Default::default(),
            agreed: Vec::new(),
        }
    }
}

impl Game {
    pub fn record_deal_response(&mut self, offer: &Rc<DealOffered>, faction: Faction, accepted: bool) {
        let key = RcKey(offer.clone());
        if !self.deals.responses.contains_key(&key) {
            // Never a duplicate: the key was just checked.
            self.deals.responses.insert_new(key.clone(), IndexMap::new());
        }

        self.deals.responses.get_mut(&key).expect("responses to this offer").insert(faction, accepted);
    }

    pub fn deal_response(&self, offer: &Rc<DealOffered>, faction: Faction) -> Option<bool> {
        let key = RcKey(offer.clone());
        match self.deals.responses.get(&key) {
            Some(responses) => responses.get(&faction).copied(),
            None => None,
        }
    }

    pub fn has_responded_to_deal(&self, offer: &Rc<DealOffered>, faction: impl Into<Option<Faction>>) -> bool {
        let faction: Option<Faction> = faction.into();
        self.deal_response(offer, faction.expect("faction")).is_some()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn deal_recipients(&self, offer: &DealOffered) -> impl Iterator<Item = Faction> {
        let initiator = offer.initiator();
        self.players
            .iter()
            .filter(move |p| {
                // offer.To is an array; a null To throws on .Length.
                p.faction != initiator && p.ally != initiator && {
                    let to = offer.to.as_ref().expect("destination");
                    to.len() == 0 || to.contains_opt(p.faction)
                }
            })
            .filter_map(move |p| p.faction)
    }

    pub fn start_deal(&mut self, deal: &Deal) {
        self.deals.agreed.push(deal.clone());
    }

    pub fn has_deal(&self, f: impl Into<Option<Faction>>, type_: DealType) -> bool {
        let f: Option<Faction> = f.into();
        self.deals.agreed.iter().any(|deal| deal.consuming_faction == f && deal.type_ == type_)
    }

    pub fn remove_ended_deals(&mut self, phase: Phase) {
        // Deals are distinct objects, so List.Remove(deal) on the snapshot removes exactly
        // the deals whose End matches.
        self.deals.agreed.retain(|deal| deal.end != Some(phase));
    }

    pub fn is_ghola(&self, l: Hero) -> bool {
        let tleilaxu = self.get_player(Some(Faction::Tleilaxu));
        l.faction() != Some(Faction::Tleilaxu)
            && tleilaxu.is_some()
            && match l.as_leader() {
                // Leaders.Contains(IHero): reference equality, so only a Leader can match.
                Some(leader) => self.player(tleilaxu.expect("tleilaxu")).leaders.contains(&leader),
                None => false,
            }
    }
}

#[derive(Clone, Debug)]
pub struct Deal {
    pub bound_faction: Option<Faction>,
    pub consuming_faction: Option<Faction>,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub benefit: i32,
    pub end: Option<Phase>,
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
            Some(p) => Deal::parameter1_territory(g, self.type_, p),
        }
    }

    pub fn parameter1_territory(g: &Game, type_: DealType, parameter: &str) -> Option<Territory> {
        match type_ {
            DealType::DontShipOrMoveTo => g.board.map.territory_lookup().find(dotnet_compat::parse::int_parse(Some(parameter))),
            _ => None,
        }
    }

    pub fn deal_contents_description(&self, g: &Game) -> Message {
        Deal::describe_deal_contents(
            g,
            self.type_,
            Some(self.text.as_deref().unwrap_or("")),
            self.benefit,
            self.end,
            Some(self.deal_parameter1.as_deref().unwrap_or("")),
        )
    }

    /// [`describe_deal_contents`](Self::describe_deal_contents),
    /// failing when the description needs a territory and `parameter1` isn't a number (where
    /// that one panics like .NET's `int.Parse`).
    pub fn try_deal_contents_description(g: &Game, type_: DealType, text: Option<&str>, benefit: i32, end: Option<Phase>, parameter1: Option<&str>) -> GameResult<Message> {
        let uses_parameter = text.map_or(true, |t| t.chars().all(char::is_whitespace));
        if uses_parameter && type_ == DealType::DontShipOrMoveTo {
            if let Err(e) = dotnet_compat::parse::try_int_parse(Some(parameter1.unwrap_or("?"))) {
                return Err(GameError::new(e.message));
            }
        }
        Ok(Deal::describe_deal_contents(g, type_, text, benefit, end, parameter1))
    }

    pub fn describe_deal_contents(g: &Game, type_: DealType, text: Option<&str>, benefit: i32, end: Option<Phase>, parameter1: Option<&str>) -> Message {
        // string.IsNullOrWhiteSpace: char.IsWhiteSpace and Rust's char::is_whitespace agree.
        let is_null_or_white_space = text.map_or(true, |t| t.chars().all(char::is_whitespace));
        if !is_null_or_white_space {
            return express!(
                express_part_if!(benefit > 0; "Receive ", Payment::of(benefit), " and "),
                text.expect("text").to_string(),
                " until ",
                end
            );
        }
        express!(
            express_part_if!(benefit > 0; "Receive ", Payment::of(benefit), " and "),
            Deal::express(type_, Deal::parameter1_territory(g, type_, parameter1.unwrap_or("?")).into()),
            " until ",
            end
        )
    }

    pub fn express(d: DealType, parameter: Part) -> Message {
        let p: Part = match parameter {
            Part::Null => "...".into(),
            other => other,
        };

        match d {
            DealType::None => express!("Custom deal"),
            DealType::DontShipOrMoveTo => express!("Don't ship or move to ", p),
            DealType::ShareBiddingPrescience => express!("Share treachery card prescience"),
            DealType::ShareResourceDeckPrescience => express!("Share prescience of the top ", Concept::Resource, " card"),
            DealType::ShareStormPrescience => express!("Share storm prescience"),
            DealType::ForfeitBattle => express!("Forfeit this battle (no weapons and defenses, lowest leader, zero dial)"),
            DealType::TellDiscardedTraitors => express!("Tell which traitors were discarded"),
        }
    }
}
