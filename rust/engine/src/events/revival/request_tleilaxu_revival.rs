use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RequestTleilaxuRevival {
    pub hero: Option<Hero>,
}

impl Default for RequestTleilaxuRevival {
    fn default() -> Self {
        RequestTleilaxuRevival {
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
        }
    }
}

impl RequestTleilaxuRevival {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Hero> {
        use either::Either;
        use std::iter::empty;
        let tleilaxu = g.get_player(Some(Faction::Tleilaxu));
        // The gholas query is built first and runs lazily; its predicate reads p.Faction per element.
        let gholas = match tleilaxu {
            Some(tleilaxu) => Either::Left(g.player(tleilaxu).leaders.iter().filter(move |l| Some(l.faction()) == g.player(p).faction).map(|l| Hero::Leader(*l))),
            None => Either::Right(empty()),
        };
        // KilledHeroes builds its list at the call; the Union is lazy.
        let killed = g.killed_heroes(p);
        crate::iter::unique(killed.into_iter().chain(gholas))
    }
}

impl EventRules for RequestTleilaxuRevival {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !Self::valid_targets(game, by.player()).any(|h| Some(h) == self.hero) {
            return Err(Rejection(express!(self.hero, " can't be revived this way")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let hero = self.hero;
        let requests = &mut game.revival_mut().required("the revival phase")?.current_revival_requests;
        // Replaces an earlier request for the same hero.
        if let Some(i) = requests.iter().position(|r| r.hero == hero) {
            requests.remove(i);
        }
        requests.push(RevivalRequest { initiator: by.faction, player: by.player, hero });

        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " request ", Faction::Tleilaxu, " revival of a leader")
    }
}
