use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AllianceOffered {
    pub target: Option<Faction>,
}

impl AllianceOffered {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        (0..g.players.len())
            .map(move |i| g.player_at(i))
            .filter(move |other| {
                p != *other && !g.player(p).have_forces_on_each_others_home_worlds(g, *other) && !AllianceOffered::has_passed_in_alliance_phase(g, *other)
            })
            .filter_map(move |other| g.player(other).faction)
    }

    fn has_passed_in_alliance_phase(g: &Game, p: PlayerId) -> bool {
        matches!(g.turn.current_phase, Phase::AllianceA | Phase::AllianceB) && g.turn.has_acted_or_passed.contains_opt(g.player(p).faction)
    }
}

impl EventRules for AllianceOffered {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        // ValidTargets dereferences a null Player only inside its lazy predicate, per element.
        if !AllianceOffered::valid_targets(game, by.player()).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid faction")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let initiator = by.faction;
        let matching_offer = game.alliances.current_alliance_offers.iter().any(|x| x.initiator == self.target && x.target == initiator);
        if matching_offer {
            game.make_alliance(initiator.required("initiator")?, self.target.required("target")?);

            // The new allies' own offers lapse.
            game.alliances.current_alliance_offers.retain(|x| x.initiator != initiator && x.initiator != self.target);

            // Offers to the new allies lapse too; otherwise their makers would wait for an answer that never comes.
            game.alliances.current_alliance_offers.retain(|x| x.target != initiator && x.target != self.target);

            game.turn.has_acted_or_passed.push(initiator.required("faction")?);
            game.turn.has_acted_or_passed.push(self.target.required("faction")?);

            game.end_nexus_if_everyone_is_done()?;
        } else {
            let m = self
                .describe(game, by)
                .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Offered, initiator, self.target))));
            game.log(parts![m]);
            game.alliances.current_alliance_offers.push(AllianceOffer { initiator, player: by.player, target: self.target });
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " offer an alliance to ", self.target)
    }
}
