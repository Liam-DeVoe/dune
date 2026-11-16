use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveryRevealed {
    pub location: Option<Location>,
    pub passed: bool,
}

impl Default for DiscoveryRevealed {
    fn default() -> Self {
        DiscoveryRevealed {
            location: Location::from_id(0),
            passed: Default::default(),
        }
    }
}

impl DiscoveryRevealed {
    pub fn applicable(g: &Game, p: PlayerId) -> bool {
        g.turn.current_main_phase == MainPhase::Collection && !Self::get_locations(g, p).next().is_none()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn get_locations(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        g.board.discoveries_on_planet
            .iter()
            .filter(move |(k, v)| g.collection.pending_discoveries.contains(&v.token) && g.player(p).occupies_territory(g, Some(k.territory())))
            .map(move |(k, _)| *k)
    }
}

impl EventRules for DiscoveryRevealed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let location = self.location;
        if !Self::get_locations(game, by.player()).any(|l| Some(l) == location) {
            return Err(Rejection(express!("Invalid location")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Location!: a null key would be ArgumentNullException; Validate guarantees a valid location.
        let Some(location) = self.location else {
            game_error!("Value cannot be null. (Parameter 'key')")
        };
        let discovery = game.board.discoveries_on_planet[&location].clone();
        if let Some(i) = game.collection.pending_discoveries.iter().position(|t| *t == discovery.token) {
            game.collection.pending_discoveries.remove(i);
        }

        if !self.passed {
            game.stone(Milestone::DiscoveryRevealed);
            game.board.discoveries_on_planet.remove(&discovery.location);

            game.log(parts![by.faction, " reveal ", discovery.token, " in ", discovery.location.territory()]);

            match discovery.token {
                DiscoveryToken::Jacurutu
                | DiscoveryToken::Shrine
                | DiscoveryToken::TestingStation
                | DiscoveryToken::Cistern
                | DiscoveryToken::ProcessingStation => {
                    let sh = game.board.map.get_discovery_stronghold(discovery.token);
                    sh.point_at(&mut game.board.map, discovery.location);
                    game.collection.just_revealed_discovery_strongholds.push(sh);
                }

                DiscoveryToken::CardStash => {
                    let card = game.draw_treachery_card();
                    let me = by.player();
                    game.player_mut(me).treachery_cards.push(card.required("card")?);
                    game.log(parts![by.faction, " draw a treachery card"]);
                    if game.player(me).hand_size_exceeded(game) {
                        game.let_factions_discard_surplus_cards();
                    }
                }

                DiscoveryToken::ResourceStash => {
                    let me = by.player();
                    let r = &mut game.player_mut(me).resources;
                    *r += 7;
                    game.log(parts![by.faction, " get ", Payment::of(7)]);
                }

                DiscoveryToken::Flight => {
                    game.collection.owner_of_flight_discovery = Some(by.faction);
                    game.log(parts![by.faction, " gain the ", discovery.token, " discovery token"]);
                }
            }
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        self.try_describe(game, by).expect("the message of an applicable DiscoveryRevealed event")
    }

    fn try_describe(&self, _game: &Game, by: By) -> GameResult<Message> {
        Ok(if self.passed {
            express!(by.faction, " don't reveal a discovery")
        } else {
            express!(by.faction, " reveal a discovery in ", self.location.required("location")?.territory())
        })
    }
}
