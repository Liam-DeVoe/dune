use crate::prelude::*;
use crate::messages::part::Part;

/// Deals offered, the responses to them, and the deals in force.
#[derive(Debug)]
pub struct Deals {
    pub offers: Vec<DealOffer>,
    pub responses: IndexMap<DealId, IndexMap<Faction, bool>>,
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

/// A deal offer: the place of its `DealOffered` in the history, which names it in views and
/// keeps apart two offers with the same terms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DealId(pub usize);

/// An open deal offer, as `DealOffered` made it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DealOffer {
    pub id: DealId,
    pub initiator: Option<Faction>,
    pub player: Option<PlayerId>,
    pub to: Option<Vec<Faction>>,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end_phase: Option<Phase>,
    pub price: u16,
    pub benefit: u16,
}

impl DealOffer {
    /// The offering player; panics for an offer without one.
    #[track_caller]
    pub fn player(&self) -> PlayerId {
        self.player.expect("the offer has a player")
    }

    /// The event that made the offer.
    pub fn event(&self) -> DealOffered {
        DealOffered {
            to: self.to.clone(),
            type_: self.type_,
            deal_parameter1: self.deal_parameter1.clone(),
            deal_parameter2: self.deal_parameter2.clone(),
            text: self.text.clone(),
            end_phase: self.end_phase,
            price: self.price,
            benefit: self.benefit,
            cancel: false,
        }
    }

    /// The submitter of the offer, as it was submitted.
    pub fn by(&self) -> By {
        By { faction: self.initiator, player: self.player }
    }

    /// The offer's log line.
    pub fn describe(&self, game: &Game) -> Message {
        self.event().describe(game, self.by())
    }

    /// The withdrawal of the offer: a copy of its history entry (submitter and time included)
    /// with `cancel` set. Anyone may submit it, and bots do for other factions' offers.
    pub fn cancellation(&self, game: &Game) -> HistoryEntry {
        let at = game.history.entries.get(self.id.0).map(|e| e.at).unwrap_or_default();
        HistoryEntry { by: self.initiator, at, event: Event::from(DealOffered { cancel: true, ..self.event() }), id: EntryId::fresh() }
    }

    /// Accepting the offer.
    pub fn acceptance(&self) -> DealAccepted {
        DealAccepted {
            bound_faction: self.initiator,
            price: self.price,
            type_: self.type_,
            deal_parameter1: self.deal_parameter1.clone(),
            deal_parameter2: self.deal_parameter2.clone(),
            text: self.text.clone(),
            benefit: self.benefit,
            end: self.end_phase,
        }
    }

    /// Declining the offer.
    pub fn rejection(&self) -> DealRejected {
        DealRejected {
            bound_faction: self.initiator,
            price: self.price,
            type_: self.type_,
            deal_parameter1: self.deal_parameter1.clone(),
            deal_parameter2: self.deal_parameter2.clone(),
            text: self.text.clone(),
            benefit: self.benefit,
            end: self.end_phase,
        }
    }

    /// Whether `offered_deal` offers the same terms (a withdrawal names the offer by its terms).
    pub fn same(&self, offered_deal: &DealOffered) -> bool {
        i32::from(offered_deal.price) == i32::from(self.price)
            && offered_deal.type_ == self.type_
            && offered_deal.deal_parameter1 == self.deal_parameter1
            && offered_deal.deal_parameter2 == self.deal_parameter2
            && offered_deal.text == self.text
            && i32::from(offered_deal.benefit) == i32::from(self.benefit)
            && offered_deal.end_phase == self.end_phase
            // C# parity: C# 14 binds `Faction[].SequenceEqual(Faction[])` to
            // MemoryExtensions.SequenceEqual on spans, where a null array is an
            // empty span. So null equals [] and nothing throws.
            && offered_deal.to.as_deref().unwrap_or(&[]) == self.to.as_deref().unwrap_or(&[])
    }

    /// Whether `accepted_deal`, submitted by `by`, accepts this offer.
    pub fn is_accepted_by(&self, accepted_deal: &DealAccepted, by: Option<Faction>) -> bool {
        self.matches(
            by,
            accepted_deal.bound_faction,
            i32::from(accepted_deal.price),
            accepted_deal.type_,
            accepted_deal.deal_parameter1.as_deref(),
            accepted_deal.deal_parameter2.as_deref(),
            accepted_deal.text.as_deref(),
            i32::from(accepted_deal.benefit),
            accepted_deal.end,
        )
    }

    /// Whether `rejected_deal`, submitted by `by`, declines this offer.
    pub fn is_rejected_by(&self, rejected_deal: &DealRejected, by: Option<Faction>) -> bool {
        self.matches(
            by,
            rejected_deal.bound_faction,
            i32::from(rejected_deal.price),
            rejected_deal.type_,
            rejected_deal.deal_parameter1.as_deref(),
            rejected_deal.deal_parameter2.as_deref(),
            rejected_deal.text.as_deref(),
            i32::from(rejected_deal.benefit),
            rejected_deal.end,
        )
    }

    pub fn matches(&self, by: Option<Faction>, bound_faction: Option<Faction>, price: i32, type_: DealType, deal_parameter1: Option<&str>, deal_parameter2: Option<&str>, text: Option<&str>, benefit: i32, end: Option<Phase>) -> bool {
        bound_faction == self.initiator
            && price == i32::from(self.price)
            && type_ == self.type_
            && deal_parameter1 == self.deal_parameter1.as_deref()
            && deal_parameter2 == self.deal_parameter2.as_deref()
            && text == self.text.as_deref()
            && benefit == i32::from(self.benefit)
            && end == self.end_phase
            && {
                let to = self.to.as_ref().expect("destination");
                to.len() == 0 || to.contains_opt(by)
            }
    }

    pub fn get_deal_description(&self, game: &Game) -> Message {
        Deal::describe_deal_contents(
            game,
            self.type_,
            self.text.as_deref(),
            i32::from(self.benefit),
            self.end_phase,
            self.deal_parameter1.as_deref(),
        )
    }
}

impl Game {
    pub fn record_deal_response(&mut self, offer: DealId, faction: Faction, accepted: bool) {
        self.deals.responses.entry(offer).or_default().insert(faction, accepted);
    }

    pub fn deal_response(&self, offer: DealId, faction: Faction) -> Option<bool> {
        self.deals.responses.get(&offer).and_then(|responses| responses.get(&faction).copied())
    }

    pub fn has_responded_to_deal(&self, offer: DealId, faction: impl Into<Option<Faction>>) -> bool {
        let faction: Option<Faction> = faction.into();
        self.deal_response(offer, faction.expect("faction")).is_some()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn deal_recipients<'a>(&'a self, offer: &'a DealOffer) -> impl Iterator<Item = Faction> + 'a {
        let initiator = offer.initiator;
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
            DealType::DontShipOrMoveTo => g.board.map.territory_lookup().find(crate::parse::int_or_panic(parameter)),
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
            if let Err(e) = crate::parse::int(parameter1.unwrap_or("?")) {
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
