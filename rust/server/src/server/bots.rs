//! The bots that play the seats without a user (protocol.md, Bots).

use treachery_protocol::UserId;
use std::time::Duration;

use treachery_bots::{BotParameters, ClassicBot, Priority};
use treachery_engine::enumerations::{MainPhase, Phase};
use treachery_engine::events::Event;
use treachery_engine::random::{self, SharedRandom, SliceRandom};
use treachery_engine::game::PlayerId;

use super::{Answer, CallError, RunningGame, Server};
use crate::protocol::ErrorCode;
use crate::ids;
use crate::timers::Timer;

/// How long the bots wait after `latest` before acting, in milliseconds, at speed 1.
pub fn bot_delay(phase: MainPhase, latest: Option<&Event>) -> u64 {
    match latest {
        Some(Event::AllyPermission(_) | Event::DealOffered(_) | Event::SetShipmentPermission(_)) => 800,
        Some(Event::Bid(_)) => 1600,
        _ if phase == MainPhase::Resurrection || phase == MainPhase::Charity => 2400,
        _ if phase == MainPhase::ShipmentAndMove || phase == MainPhase::Battle => 9600,
        _ => 4800,
    }
}

/// The index in `g.bots` of the bot of the player's faction, created the first time that
/// faction's bot acts.
fn bot_for(g: &mut RunningGame, player: PlayerId) -> usize {
    let faction = g.game.player(player).faction;
    let index = match g.bots.iter().position(|(f, _)| *f == faction) {
        Some(i) => {
            g.bots[i].1.set_player(player);
            i
        }
        None => {
            let random = SharedRandom::from_seed(ids::new_seed());
            g.bots.push((faction, ClassicBot::new(player, BotParameters::for_faction(faction), random)));
            g.bots.len() - 1
        }
    };
    index
}

impl Server {
    /// Schedules the bots' next turn, replacing any scheduled before. Nothing at speed 0.
    pub(super) fn schedule_bots(&mut self, game: &str) {
        let Some(g) = self.games.get_mut(game) else { return };
        let speed = g.participation.bots_speed;
        if speed <= 0 || g.scene {
            return;
        }
        let delay = bot_delay(g.game.turn.current_main_phase, g.game.latest_event_0().as_ref().map(|e| &e.event)) / speed as u64;
        g.bot_turn += 1;
        self.services.timers.schedule(Duration::from_millis(delay), Timer::BotTurn { game: game.to_string(), turn: g.bot_turn });
    }

    /// Runs the bots' turn now, at any speed but 0. For the game's players.
    pub(super) async fn nudge_bots(&mut self, user: UserId, game: &str) -> Answer<()> {
        let game = self.player_of(user, game)?;
        match self.bot_turn(&game).await {
            Ok(()) | Err(CallError::Refused(..)) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Each bot, in random order, is asked for its most urgent action, then the next level, and
    /// the first action found is executed. Bots end phases themselves only when the game has no
    /// host and someone watches; otherwise a turn in which no bot acts counts as the bots
    /// passing in the current pause.
    pub(super) async fn bot_turn(&mut self, game: &str) -> Result<(), CallError> {
        let Some(g) = self.games.get_mut(game) else { return Ok(()) };
        let (speed, no_host) = (g.participation.bots_speed, g.participation.hosts.is_empty());
        if speed <= 0 || g.game.turn.current_phase <= Phase::AwaitingPlayers || g.scene {
            return Ok(());
        }
        let bots_act_as_hosts = no_host && g.participation.number_of_observers() > 0;
        let mut bots: Vec<PlayerId> = g.game.player_indices().filter(|&p| g.game.is_bot(p)).collect();
        bots.shuffle(&mut random::new_rng(ids::new_seed()));
        let options: Vec<_> = bots.iter().map(|&p| g.game.get_applicable_events(Some(p), bots_act_as_hosts)).collect();

        let levels = if bots_act_as_hosts { 5 } else { 4 };
        for level in 0..levels {
            for (i, &player) in bots.iter().enumerate() {
                let g = self.games.get_mut(game).expect("present");
                let options = &options[i];
                let index = bot_for(g, player);
                let RunningGame { bots, game: state, .. } = g;
                let bot = &mut bots[index].1;
                let event = bot.decide(state, Priority::ALL[level], options);
                if let Some(event) = event {
                    return match Box::pin(self.execute(game, event, level == 4, None, true)).await {
                        Err(CallError::Refused(ErrorCode::InvalidGameEvent, _)) => Ok(()),
                        r => r,
                    };
                }
            }
        }
        if !bots.is_empty() {
            Box::pin(self.update_pause(game, None, Some(Box::new(|pause: &mut super::Pause| pause.bots_passed = true)))).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays() {
        assert_eq!(bot_delay(MainPhase::Battle, None), 9600);
        assert_eq!(bot_delay(MainPhase::Charity, None), 2400);
        assert_eq!(bot_delay(MainPhase::Bidding, None), 4800);
        let bid: treachery_engine::events::HistoryEntry = treachery_engine::json::deserialize(r#"{"$type":"Bid","Initiator":20,"Amount":1}"#).unwrap();
        assert_eq!(bot_delay(MainPhase::Bidding, Some(&bid.event)), 1600);
    }
}
