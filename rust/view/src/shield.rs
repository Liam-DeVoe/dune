//! The viewer's own faction panel (`PlayerPanel.razor`), the notekeeping panel
//! (`NoteComponent.razor`) and which hand cards can be played now (`CardPlays.cs`).

use treachery_engine::events::GameEventType;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::prelude::*;

use crate::schema::*;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

pub fn hero_ref(h: Hero) -> HeroRef {
    match h {
        Hero::Leader(l) => HeroRef::Leader { id: l.id() },
        Hero::Messiah => HeroRef::Messiah,
        Hero::TreacheryCard(c) => HeroRef::TreacheryCard { id: c.id() },
    }
}

/// `Game.Skill(h)`, when there is one.
fn skill_of(game: &Game, h: Hero) -> Option<String> {
    Some(game.skill(Some(h))).filter(|s| *s != LeaderSkill::None).map(name)
}

fn hero_card(game: &Game, h: Hero, revealed: bool) -> HeroCard {
    HeroCard { hero: hero_ref(h), revealed, skill: skill_of(game, h) }
}

/// `CardPlays.Plays`: the actions that play a treachery card from the faction panel, and the
/// card type each plays.
fn card_type_played_by(t: GameEventType) -> Option<TreacheryCardType> {
    use GameEventType as E;
    Some(match t {
        E::Karma
        | E::KarmaChoamDiscard
        | E::KarmaFreeRevival
        | E::KarmaHandSwapInitiated
        | E::KarmaHmsMovement
        | E::KarmaMonster
        | E::KarmaEcazDial
        | E::KarmaPrescience
        | E::KarmaRevivalPrevention
        | E::KarmaShipmentPrevention
        | E::KarmaRicheseBuy => TreacheryCardType::Karma,
        E::AmalPlayed => TreacheryCardType::Amal,
        E::Caravan => TreacheryCardType::Caravan,
        E::ClairVoyancePlayed => TreacheryCardType::Clairvoyance,
        E::DiscardedSearchedAnnounced => TreacheryCardType::SearchDiscarded,
        E::DistransUsed => TreacheryCardType::Distrans,
        E::HarvesterPlayed => TreacheryCardType::Harvester,
        E::JuicePlayed => TreacheryCardType::Juice,
        E::MetheorPlayed => TreacheryCardType::Metheor,
        E::PortableAntidoteUsed => TreacheryCardType::PortableAntidote,
        E::RaiseDeadPlayed => TreacheryCardType::RaiseDead,
        E::RecruitsPlayed => TreacheryCardType::Recruits,
        E::ResidualPlayed => TreacheryCardType::Residual,
        E::StormSpellPlayed => TreacheryCardType::StormSpell,
        E::ThumperPlayed => TreacheryCardType::Thumper,
        _ => return None,
    })
}

/// `CardPlays.CardsFor(action, game, player)`: the cards in hand that play this action.
pub fn cards_for(game: &Game, p: PlayerIndex, action: GameEventType) -> Vec<TreacheryCard> {
    match card_type_played_by(action) {
        None => vec![],
        Some(TreacheryCardType::Karma) => Karma::valid_karma_cards(game, p).collect(),
        Some(t) => game.player(p).treachery_cards.iter().copied().filter(|c| c.type_() == t).collect(),
    }
}

/// `CardPlays.PlaysCard` for every applicable action: the actions that play `card`.
pub fn playable_with(game: &Game, p: PlayerIndex, card: TreacheryCard, actions: &[GameEventType]) -> Vec<String> {
    actions.iter().filter(|a| cards_for(game, p, **a).contains(&card)).map(|a| a.name().to_string()).collect()
}

fn messiah_state(game: &Game, p: PlayerIndex) -> Option<MessiahState> {
    let pl = game.player(p);
    (pl.faction == Faction::Atreides && game.applicable(Rule::AtreidesMessiah))
        .then(|| MessiahState { forces_killed_in_battle: pl.total_forces_killed_in_battle, available: pl.messiah_available(game) })
}

/// `PlayerPanel.razor` for the viewer's own seat. `actions` are the viewer's applicable events
/// (`Client.Actions`).
pub fn private_player(game: &Game, p: PlayerIndex, actions: &[GameEventType]) -> PrivatePlayer {
    let pl = game.player(p);
    let after_bene_gesserit_prediction = game.current_phase > Phase::BeneGesseritPredicting;
    // PlayerPanel.StillPickingTraitor: while traitors are being picked, a player who picks still
    // holds all the dealt candidates, and the panel shows none of them until they have chosen.
    let still_picking_traitor = game.current_phase == Phase::SelectingTraitors
        && pl.faction != Faction::Harkonnen
        && pl.faction != Faction::Tleilaxu
        && !game.has_acted_or_passed.contains(&pl.faction);
    PrivatePlayer {
        faction: name(pl.faction),
        resources: pl.resources,
        hand: pl.treachery_cards.iter().map(|c| HandCard { id: c.id(), playable_with: playable_with(game, p, *c, actions) }).collect(),
        card_set_aside: game.get_card_set_aside_for_bid(p).map(|c| c.id()),
        leaders: pl
            .leaders
            .iter()
            .map(|l| {
                let h = Hero::Leader(*l);
                OwnLeader { id: l.id(), alive: game.is_alive(h), skilled: game.is_skilled(Some(h)), skill: skill_of(game, h) }
            })
            .collect(),
        traitors: if still_picking_traitor {
            vec![]
        } else {
            pl.traitors.iter().map(|h| hero_card(game, *h, pl.revealed_traitors.contains(h))).collect()
        },
        face_dancers: pl.face_dancers.iter().map(|h| hero_card(game, *h, pl.revealed_face_dancers.contains(h))).collect(),
        predicted_faction: (pl.faction == Faction::BeneGesserit && after_bene_gesserit_prediction).then(|| name(pl.predicted_faction)),
        predicted_turn: (pl.faction == Faction::BeneGesserit && after_bene_gesserit_prediction).then_some(pl.predicted_turn),
        nexus: (pl.nexus != Faction::None).then(|| name(pl.nexus)),
        // PlayerPanel.razor:96: after the storm is dialled, with storm prescience.
        next_storm_moves: (game.current_phase > Phase::DiallingStorm && game.has_storm_prescience(Some(p))).then_some(game.next_storm_moves),
        messiah: messiah_state(game, p),
        unplaced_terror: (pl.faction == Faction::Moritani).then(|| game.unplaced_terror_tokens.iter().map(|t| name(*t)).collect()),
        card_just_won: game.card_just_won.filter(|c| game.known_cards(p).contains(c)).map(|c| c.id()),
    }
}

/// `NoteComponent.razor`, shown when `Game.AssistedNoteKeepingEnabled(Player)`: every other
/// player, in seat order (the C# panel sorts them by the skin's faction names, which the client
/// does).
pub fn notes(game: &Game, me: PlayerIndex) -> Option<NotesView> {
    if !game.assisted_note_keeping_enabled(Some(me)) {
        return None;
    }
    let known = &game.player(me).known_cards;
    let players = game
        .player_indices()
        .filter(|p| *p != me)
        .map(|p| {
            let pl = game.player(p);
            NotesPlayer {
                faction: name(pl.faction),
                resources: pl.resources,
                bribes: pl.bribes,
                resources_after_bidding: pl.resources_after_bidding,
                cards: game.cards_owned_by(p).iter().map(|c| known.contains(c).then(|| c.id())).collect(),
                revealed_traitors: pl.revealed_traitors.iter().map(|h| hero_ref(*h)).collect(),
                revealed_face_dancers: pl.revealed_face_dancers.iter().map(|h| hero_ref(*h)).collect(),
                victory_points: game.number_of_victory_points(p, true),
                messiah: messiah_state(game, p),
            }
        })
        .collect();
    Some(NotesView { players })
}
