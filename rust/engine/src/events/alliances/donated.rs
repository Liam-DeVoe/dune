use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Donated {
    pub target: Option<Faction>,
    pub resources: i32,
    pub from_bank: bool,
    pub card: Option<TreacheryCard>,
}

impl Default for Donated {
    fn default() -> Self {
        Donated {
            target: Default::default(),
            resources: Default::default(),
            from_bank: false,
            card: None,
        }
    }
}

impl Donated {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        // `p` is dereferenced inside the predicate, per element.
        g.players
            .iter()
            .filter(move |x| x.faction != g.player(p).faction && (g.player(p).is(Faction::Emperor) || x.faction != g.player(p).ally))
            .filter_map(move |x| x.faction)
    }

    pub fn may_donate(g: &Game, p: PlayerId) -> bool {
        if g.has_bid_to_pay(p) {
            return false;
        }

        true
    }

    pub fn min_amount(g: &Game, p: PlayerId) -> i32 {
        if !g.applicable(Rule::CardsCanBeTraded) {
            return i32::from(g.player(p).resources).min(1);
        }
        0
    }

    pub fn max_amount(game: &Game, p: PlayerId) -> i32 {
        i32::from(game.player(p).resources)
    }
}

impl EventRules for Donated {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.from_bank {
            return Ok(());
        }

        // Without a player (a seatless host), the only check that needs one is whether there is a
        // bid to pay, so that case is refused here.
        let may_donate = match by.player {
            Some(p) => Donated::may_donate(game, p),
            None => {
                if game.phase() == Phase::Bidding && game.current_bid().is_some() {
                    return Err(Rejection(express!("You are not playing")));
                }
                true
            }
        };
        if !may_donate {
            return Err(Rejection(express!("You currently have an outstanding bid")));
        }
        if self.card.is_none() && self.resources <= 0 {
            return Err(Rejection(express!("Invalid amount")));
        }
        // A negative amount with a card would take spice from the target (behavior-changes.md,
        // "Turns, sectors and amounts out of range").
        if !self.from_bank && self.resources < 0 {
            return Err(Rejection(express!("Invalid amount")));
        }
        if i32::from(game.player(by.player()).resources) < self.resources {
            return Err(Rejection(express!("You can't give that much")));
        }
        if by.faction != Some(Faction::Emperor) && self.target == game.player(by.player()).ally {
            return Err(Rejection(express!("You can't bribe your ally")));
        }
        if by.faction != Some(Faction::Emperor) && game.applicable(Rule::DisableResourceTransfers) {
            return Err(Rejection(express!(Concept::Resource, " transfers are disabled by house rule")));
        }
        if !self.from_bank
            && (game.board.choam_economics_status == Some(ChoamEconomicsStatus::Double) || game.board.choam_economics_status == Some(ChoamEconomicsStatus::DoubleFlipped))
        {
            // Known quirk (known-issues.md): the message is built but never returned, so bribes
            // are allowed during Double Inflation.
            let _ = express!("No bribes can be made during Double Inflation");
        }

        let target_player = game.get_player(self.target);
        if self.card.is_some() && !game.player(target_player.expect("target player")).has_room_for_cards(game) {
            return Err(Rejection(express!("Target faction's hand is full")));
        }
        // Spice still owed for a bid can't be given away, or paying the bid would take the
        // player below zero (escapes.md, "Forces below zero").
        if !game.may_give_away_spice(by.player(), self.resources) {
            return Err(Rejection(express!("You need that spice to pay your bid")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // `GetPlayer(Target)!` does not throw; a null target throws where it is dereferenced.
        let target = game.get_player(self.target);

        if !self.from_bank {
            match target {
                Some(target) => game.exchange_resources_in_bribe(by.player(), target, self.resources)?,
                None => {
                    // ExchangeResourcesInBribe(Player, null, Resources): the payer pays first.
                    let me = by.player();
                    let from = game.player_mut(me);
                    crate::game::amounts::spend(&mut from.resources, self.resources, "spice")?;
                    let _ = game.bribes_during_mentat();
                    game_error!("A required value is missing");
                }
            }
            let target = target.required("target")?;

            if let Some(card) = self.card {
                let me = by.player();
                {
                    let cards = &mut game.player_mut(me).treachery_cards;
                    if let Some(i) = cards.iter().position(|c| *c == card) {
                        cards.remove(i);
                    }
                }
                Game::register_known_by_player(game, me, Some(card));
                game.player_mut(target).treachery_cards.push(card);

                let initiator = by.faction;
                for i in 0..game.players.len() {
                    let p = game.player_at(i);
                    if !game.player(p).is(initiator) && p != target {
                        let my_cards = game.player(me).treachery_cards.clone();
                        Game::unregister_all_known_by_player(game, p, &my_cards);
                        let target_cards = game.player(target).treachery_cards.clone();
                        Game::unregister_all_known_by_player(game, p, &target_cards);
                    }
                }
            }

            let m = self.describe(game, by).with_log(AsideLog::aside());
            game.log(vec![m.into()]);
            game.stone(Milestone::Bribe);
        } else if self.resources < 0 {
            let abs = self.resources.abs();
            let resources_to_take = abs.min(i32::from(game.player(target.required("target")?).resources));
            let target = target.required("target")?;
            game.log(vec![
                    "Host puts ".into(),
                    Payment::of(resources_to_take).into(),
                    " from ".into(),
                    self.target.into(),
                    " into the ".into(),
                    Concept::Resource.into(),
                    " Bank".into(),
                ],
            );
            let t = game.player_mut(target);
            crate::game::amounts::spend(&mut t.resources, resources_to_take, "spice")?;
        } else {
            game.log(vec![
                    "Host gives ".into(),
                    self.target.into(),
                    Payment::of(self.resources).into(),
                    " from the ".into(),
                    Concept::Resource.into(),
                    " Bank".into(),
                ],
            );
            let t = game.player_mut(target.required("target")?);
            crate::game::amounts::gain(&mut t.resources, self.resources, "spice")?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(
            by.faction,
            " give ",
            Payment::of(self.resources),
            express_part_if!(self.card.is_some(); " and a card"),
            " to ",
            self.target
        )
    }
}
