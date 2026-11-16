use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AcceptOrCancelTleilaxuRevival {
    pub cancel: bool,
    pub price: i32,
    pub hero: Option<Hero>,
}

impl Default for AcceptOrCancelTleilaxuRevival {
    fn default() -> Self {
        AcceptOrCancelTleilaxuRevival {
            cancel: Default::default(),
            price: Default::default(),
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
        }
    }
}

impl AcceptOrCancelTleilaxuRevival {
    pub fn is_denial(&self) -> bool {
        self.price == i32::MAX
    }
}

impl EventRules for AcceptOrCancelTleilaxuRevival {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        match self.hero {
            None => {
                let requests = game.revival.current_revival_requests.clone();
                for r in requests.iter() {
                    let Some(h) = r.hero else { game_error!("A revival request has no leader") };
                    game.revival.early_revivals_offers.insert_new(h, i32::MAX);
                }

                game.revival.current_revival_requests.clear();
            }
            Some(hero) => {
                game.revival.early_revivals_offers.shift_remove(&hero);

                if !self.cancel {
                    game.revival.early_revivals_offers.insert_new(hero, self.price);
                }

                // C# removed the first request for the hero, by reference.
                if let Some(i) = game.revival.current_revival_requests.iter().position(|r| r.hero == Some(hero)) {
                    game.revival.current_revival_requests.remove(i);
                }
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        let Some(hero) = self.hero else {
            return express!(by.faction, " deny all outstanding requests for early revival");
        };

        if !self.cancel {
            if self.is_denial() {
                return express!(by.faction, " deny ", hero.faction(), " early revival of a leader");
            }
            return express!(by.faction, " offer ", hero.faction(), " revival of a leader for ", Payment::of(self.price));
        }

        express!(by.faction, " cancel their revival offer to ", hero.faction())
    }
}
