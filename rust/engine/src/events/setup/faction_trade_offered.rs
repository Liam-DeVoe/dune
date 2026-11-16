use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct FactionTradeOffered {
    pub target: Option<Faction>,
}

impl FactionTradeOffered {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        let factions: Vec<Faction> = g.rules.factions_in_play.clone();
        // `p.Faction` is read inside the predicate, per element.
        crate::iter::unique(factions.into_iter().chain(g.players.iter().filter_map(|p| p.faction))).filter(move |f| Some(*f) != g.player(p).faction)
    }

    pub fn remove_invalid_trade_offers(&self, game: &mut Game, by: By) {
        let initiator = by.faction;
        let target = self.target;
        // C# removed each matching offer by reference; no two offers are equal, so this removes the same ones.
        game.setup_state().current_trade_offers.retain(|x| !(x.initiator == initiator || x.initiator == target));
    }
}

impl EventRules for FactionTradeOffered {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        // `p.Faction` is read inside the lazy predicate, so a null Player throws only once the union has an element.
        if !Self::valid_targets(game, by.player()).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid target")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.setup().is_none() {
            game_error!("Not in the setup phase");
        }
        if !game.is_playing(self.target) {
            game.log(vec![by.faction.into(), " switch to ".into(), self.target.into()]);

            game.player_mut(by.player()).faction = self.target;
        } else {
            let initiator = by.faction;
            let target_faction = self.target;
            let r#match = game
                .setup_state().current_trade_offers
                .iter()
                .find(|m| m.initiator == target_faction && m.target == initiator)
                .cloned();
            if let Some(r#match) = r#match {
                game.log(vec![by.faction.into(), " and ".into(), r#match.initiator.into(), " traded factions".into()],
                );
                let target = game.get_player(self.target);
                // (target.Faction, Player.Faction) = (Player.Faction, target.Faction);
                let me = by.player();
                let my_faction = game.player(me).faction;
                let target = target.required("target")?;
                let target_faction = game.player(target).faction;
                game.player_mut(target).faction = my_faction;
                game.player_mut(me).faction = target_faction;
                self.remove_invalid_trade_offers(game, by);
            } else {
                let message = self.describe(game, by);
                game.log(vec![message.into()]);
                if !game.setup_state().current_trade_offers.iter().any(|o| o.initiator == initiator && o.target == target_faction) {
                    game.setup_state().current_trade_offers.push(TradeOffer { initiator, target: target_faction });
                }
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " offer to trade factions with ", self.target)
    }
}
