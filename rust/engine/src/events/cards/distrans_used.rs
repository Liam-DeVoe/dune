use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DistransUsed {
    pub target: Option<Faction>,
    pub card: Option<TreacheryCard>,
}

impl DistransUsed {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(game: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().filter(move |c| c.type_() != TreacheryCardType::Distrans)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players
            .iter()
            .enumerate()
            .filter(move |(i, target)| g.player_at(*i) != p && target.has_room_for_cards(g))
            .filter_map(move |(_, target)| target.faction)
    }

    pub fn can_be_played(game: &Game, player: PlayerId) -> bool {
        game.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Distrans)
            && !DistransUsed::valid_cards(game, player).next().is_none()
            && !DistransUsed::valid_targets(game, player).next().is_none()
    }

    pub fn check_if_bidding_for_player_should_be_skipped(game: &mut Game, player: PlayerId, had_room_for_cards: bool) {
        // Known quirk (known-issues.md): during black market bidding this also plays a regular
        // `Bid` pass, not a `BlackMarketBid`. The pass is played as an event, so it runs the
        // event's tasks and is added to the history.
        let pass = Bid { passed: true, ..Bid::default() };
        if game.phase() == Phase::BlackMarketBidding && had_room_for_cards && !game.player(player).has_room_for_cards(game) && BlackMarketBid::may_be_played(game, player) {
            let faction = game.player(player).faction.expect("faction");
            let _ = game.play(HistoryEntry::new(faction, pass), false, false);
        } else if game.phase() == Phase::Bidding && had_room_for_cards && !game.player(player).has_room_for_cards(game) && Bid::may_be_played(game, Some(player)) {
            let faction = game.player(player).faction.expect("faction");
            let _ = game.play(HistoryEntry::new(faction, pass), false, false);
        }
    }
}

impl EventRules for DistransUsed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !DistransUsed::valid_cards(game, by.player()).any(|c| Some(c) == self.card) {
            return Err(Rejection(express!("Invalid card")));
        }
        if !DistransUsed::valid_targets(game, by.player()).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid target")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let target = game.get_player(self.target);
        let target_had_room_for_cards = game.player(target.required("target")?).has_room_for_cards(game);
        let target = target.required("target")?;
        let me = by.player();
        game.discard(me, TreacheryCardType::Distrans)?;
        let card = self.card;
        crate::game::remove_card(&mut game.player_mut(me).treachery_cards, card);
        Game::register_known_by_player(game, me, card);
        game.player_mut(target).treachery_cards.push(card.required("card")?);

        if !game.player(me).treachery_cards.is_empty() {
            let initiator = by.faction;
            let others: Vec<PlayerId> = game.player_indices().filter(|p| !game.player(*p).is(initiator) && *p != target).collect();
            for p in others {
                let my_cards = game.player(me).treachery_cards.clone();
                Game::unregister_all_known_by_player(game, p, &my_cards);
                let target_cards = game.player(target).treachery_cards.clone();
                Game::unregister_all_known_by_player(game, p, &target_cards);
            }
        }

        let m = self.describe(game, by);
        game.log_event(m);
        DistransUsed::check_if_bidding_for_player_should_be_skipped(game, target, target_had_room_for_cards);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::Distrans, " to give a card to ", self.target)
    }
}
