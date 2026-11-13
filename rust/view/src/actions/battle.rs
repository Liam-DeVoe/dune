//! Action forms of the battle area. Each builder holds the results of the engine helpers its form
//! needs, and each default draft is the form's starting inputs after its widgets have adjusted
//! them: a required picker (image choice, radio, select) takes the first value when its value
//! isn't one of the choices, and a number picker clamps into its range.
//!
//! Option values that go into the event (enum members, ids) are sent as the numbers of the
//! savegame format, with the member name for images and the skin's label for text, so the client
//! needs no enum tables.

use treachery_engine::events::GameEventType;
use treachery_engine::prelude::*;
use treachery_engine::compat::net_enum::NetEnum;

use crate::build::{event, validate, ResolvedViewer};
use crate::describe::Describer;
use crate::expr;
use crate::schema::*;
use crate::shield::hero_ref;
use super::widgets::draft_json;

// ------------------------------------------------------------------------------------------
// Dispatch.
// ------------------------------------------------------------------------------------------

/// The event types whose forms this module builds.
pub const TYPES: &[GameEventType] = &[
    GameEventType::AuditCancelled,
    GameEventType::Audited,
    GameEventType::Battle,
    GameEventType::BattleConcluded,
    GameEventType::BattleInitiated,
    GameEventType::BeneGesseritBattleAnnouncement,
    GameEventType::CardsDetermined,
    GameEventType::ClairVoyanceAnswered,
    GameEventType::ClairVoyancePlayed,
    GameEventType::Diplomacy,
    GameEventType::FaceDanced,
    GameEventType::FaceDancerReplaced,
    GameEventType::FaceDancerRevealed,
    GameEventType::HMSAdvantageChosen,
    GameEventType::JuicePlayed,
    GameEventType::LoserConcluded,
    GameEventType::Planetology,
    GameEventType::Prescience,
    GameEventType::Retreat,
    GameEventType::RockWasMelted,
    GameEventType::TakeLosses,
    GameEventType::ThoughtAnswered,
    GameEventType::Thought,
    GameEventType::TreacheryCalled,
    GameEventType::Voice,
];

/// The default draft of a battle form, or None when `t` is not one of this module's events.
pub fn default_draft(game: &Game, v: &ResolvedViewer, faction: Faction, t: GameEventType) -> Option<Option<GameEvent>> {
    if !TYPES.contains(&t) {
        return None;
    }
    let Some(p) = v.player else { return Some(None) };
    Some(Some(match t {
        GameEventType::AuditCancelled => {
            let mut e = AuditCancelled::new(game, faction);
            e.cancelled = true;
            event(e)
        }
        GameEventType::Audited => event(Audited::new(game, faction)),
        GameEventType::Battle => return Some(Some(battle_default(game, p, faction))),
        GameEventType::BattleConcluded => {
            let mut e = BattleConcluded::new(game, faction);
            e.set_decision_to_capture(CaptureDecision::Kill);
            e.set_traitor_to_replace(None);
            e.set_new_traitor(None);
            e.set_discarded_cards(&[]);
            if game.battle_winner == Some(faction) {
                if BattleConcluded::may_capture_or_kill(game, Some(p)) {
                    let decisions = BattleConcluded::valid_capture_decisions(game);
                    if !decisions.contains(&CaptureDecision::Kill) {
                        e.set_decision_to_capture(decisions.first().copied().unwrap_or_default());
                    }
                }
                if let Some(loser) = loser(game) {
                    e.stolen_token = game.player(loser).tech_tokens.first().copied().unwrap_or(TechToken::None);
                }
            }
            event(e)
        }
        GameEventType::BattleInitiated => {
            let mut e = BattleInitiated::new(game, faction);
            let fights = Battle::battles_to_be_fought(game, p, false);
            if let Some(first) = fights.first() {
                e.target = first.faction;
                let t = fights.iter().find(|f| f.faction == first.faction).map(|f| f.territory);
                e.set_territory(game, t);
            }
            event(e)
        }
        GameEventType::BeneGesseritBattleAnnouncement => {
            let mut e = BeneGesseritBattleAnnouncement::new(game, faction);
            e.set_territory(game, None);
            event(e)
        }
        GameEventType::CardsDetermined => {
            let mut e = CardsDetermined::new(game, faction);
            e.set_treachery_cards(&game.treachery_deck.as_ref().map(|d| d.items.clone()).unwrap_or_default());
            e.set_richese_cards(&game.richese_cache.clone());
            event(e)
        }
        GameEventType::ClairVoyanceAnswered => {
            let mut e = ClairVoyanceAnswered::new(game, faction);
            e.answer = ClairVoyanceAnswer::Yes;
            event(e)
        }
        GameEventType::ClairVoyancePlayed => {
            let mut e = ClairVoyancePlayed::new(game, faction);
            e.target = ClairVoyancePlayed::valid_targets(game, p).next().unwrap_or_default();
            e.question = ClairVoyancePlayed::valid_questions(game, e.target).next().unwrap_or_default();
            let (a, b) = clairvoyance_defaults(game, e.question, d_none());
            e.question_parameter1 = a;
            e.question_parameter2 = b;
            event(e)
        }
        GameEventType::Diplomacy => {
            let mut e = Diplomacy::new(game, faction);
            e.set_card(Diplomacy::valid_cards(game, p).first().copied());
            event(e)
        }
        GameEventType::FaceDanced => {
            let mut e = FaceDanced::new(game, faction);
            let shows_pickers = FaceDanced::maximum_number_of_forces(game, p) > 0;
            let sources: Vec<Location> = if shows_pickers { FaceDanced::valid_source_locations(game, p).collect() } else { vec![] };
            let targets: Vec<Location> = if shows_pickers { FaceDanced::valid_target_locations(game).collect() } else { vec![] };
            let zero = |ls: &[Location]| {
                let mut d: NetDictionary<Location, Battalion> = NetDictionary::new();
                for l in ls {
                    d.add_with(*l, Battalion::new(faction, 0, 0, *l), |k| ExpressionElement::from(*k).to_cs_string());
                }
                d
            };
            e.force_locations = Some(PlacementEvent::force_locations_string(game, &zero(&sources)));
            e.set_target_force_locations(game, &zero(&targets));
            event(e)
        }
        GameEventType::FaceDancerReplaced => {
            let mut e = FaceDancerReplaced::new(game, faction);
            e.passed = false;
            e.set_selected_dancer(FaceDancerReplaced::valid_face_dancers(game, p).next());
            event(e)
        }
        GameEventType::FaceDancerRevealed => {
            // ConfirmedResult: reveal. Without a face dancer to call only "No" is drawn.
            let mut e = FaceDancerRevealed::new(game, faction);
            e.passed = false;
            event(e)
        }
        GameEventType::HMSAdvantageChosen => {
            let mut e = HMSAdvantageChosen::new(game, faction);
            e.advantage = HMSAdvantageChosen::valid_advantages(game, p).first().copied().unwrap_or_default();
            event(e)
        }
        GameEventType::JuicePlayed => {
            let mut e = JuicePlayed::new(game, faction);
            e.type_ = JuicePlayed::valid_types(game, p).first().copied().unwrap_or_default();
            event(e)
        }
        GameEventType::LoserConcluded => {
            let mut e = LoserConcluded::new(game, faction);
            e.set_kept_card(None);
            e.karma_forced_kept_card_decision = LoserConcluded::KARMA_NO;
            e.set_forced_kept_or_discarded_cards(&[]);
            event(e)
        }
        GameEventType::Planetology => {
            let mut e = Planetology::new(game, faction);
            e.add_one_to_movement = true;
            event(e)
        }
        GameEventType::Prescience => {
            let mut e = Prescience::new(game, faction);
            e.aspect = Prescience::valid_aspects(game, p).first().copied().unwrap_or_default();
            event(e)
        }
        GameEventType::Retreat => {
            let mut e = Retreat::new(game, faction);
            e.set_location(game, None);
            let (f, s) = retreat_defaults(game, p);
            e.forces = f;
            e.special_forces = s;
            event(e)
        }
        GameEventType::RockWasMelted => {
            let mut e = RockWasMelted::new(game, faction);
            e.kill = true;
            event(e)
        }
        GameEventType::TakeLosses => event(TakeLosses::new(game, faction)),
        GameEventType::ThoughtAnswered => {
            let mut e = ThoughtAnswered::new(game, faction);
            e.set_card(ThoughtAnswered::valid_cards(game, p).first().copied());
            event(e)
        }
        GameEventType::Thought => {
            let mut e = Thought::new(game, faction);
            e.set_card(Thought::valid_cards(game).next());
            event(e)
        }
        GameEventType::TreacheryCalled => {
            // The form calls the traitor. It is shown only when the player may call; otherwise
            // the client passes for them.
            let mut e = TreacheryCalled::new(game, faction);
            e.traitor_called = true;
            event(e)
        }
        GameEventType::Voice => {
            let mut e = Voice::new(game, faction);
            e.must = true;
            e.type_ = voice_types(game, &d_none()).first().map(|t| TreacheryCardType(t.value)).unwrap_or_default();
            event(e)
        }
        _ => return None,
    }))
}

/// The form's options for `draft` and the validation of the draft the form would send, or None
/// when the draft is not one of this module's events. A battle plan draft is first adjusted the
/// way the form adjusts its inputs (an invalid leader or card is dropped).
pub fn options_and_validation(game: &Game, v: &ResolvedViewer, draft: &GameEvent, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    if !TYPES.contains(&draft.event_type()) {
        return None;
    }
    let Some(p) = v.player else { return Some((None, None)) };
    let faction = game.player(p).faction;
    let dj = draft_json(draft);
    let options = match draft {
        GameEvent::AuditCancelled(_) => ActionOptions::AuditCancelled(AuditCancelledOptions {
            cost: AuditCancelled::cost_game(game),
            cards: AuditCancelled::get_number_of_cards_that_may_be_audited(game), draft: dj.clone() }),
        GameEvent::Audited(_) => ActionOptions::Audited(AuditedOptions { cards: game.audited_cards.iter().map(|c| c.id()).collect(), draft: dj.clone() }),
        GameEvent::Battle(b) => {
            let (options, normalized) = battle_options(game, p, b, d);
            return Some((Some(ActionOptions::Battle(options)), validate(game, event(normalized), d)));
        }
        GameEvent::BattleConcluded(_) => ActionOptions::BattleConcluded(battle_concluded_options(game, p, d, dj.clone())),
        GameEvent::BattleInitiated(_) => ActionOptions::BattleInitiated(BattleInitiatedOptions {
            fights: Battle::battles_to_be_fought(game, p, false)
                .iter()
                .map(|f| BattleInitiatedFight { faction: enum_choice(f.faction, d), territory: f.territory.id(), territory_label: d.describe(&f.territory.into()) })
                .collect(), draft: dj.clone() }),
        GameEvent::BeneGesseritBattleAnnouncement(_) => ActionOptions::BeneGesseritBattleAnnouncement(BeneGesseritBattleAnnouncementOptions {
            territories: BeneGesseritBattleAnnouncement::valid_territories(game, p).map(|t| t.id()).collect(),
            locations: BeneGesseritBattleAnnouncement::valid_territories(game, p).flat_map(|t| t.locations().iter().copied()).map(|l| l.id()).collect(),
            draft: dj.clone(),
        }),
        GameEvent::CardsDetermined(_) => ActionOptions::CardsDetermined(CardsDeterminedOptions {
            treachery_cards: game.treachery_deck.as_ref().map(|d| d.items.iter().map(|c| c.id()).collect()).unwrap_or_default(),
            richese_cards: game.richese_cache.iter().map(|c| c.id()).collect(), draft: dj.clone() }),
        GameEvent::ClairVoyanceAnswered(_) => {
            let Some(latest) = game.latest_clairvoyance.as_ref() else { return Some((None, validate(game, draft.clone(), d))) };
            ActionOptions::ClairVoyanceAnswered(ClairVoyanceAnsweredOptions {
                questioner: latest.initiator().to_string(),
                question: expr::message(&latest.get_question(game), d), draft: dj.clone() })
        }
        GameEvent::ClairVoyancePlayed(e) => ActionOptions::ClairVoyancePlayed(clairvoyance_options(game, p, e.target, d, dj.clone())),
        GameEvent::Diplomacy(_) => ActionOptions::Diplomacy(BattleCardsOptions { cards: Diplomacy::valid_cards(game, p).iter().map(|c| c.id()).collect(), draft: dj.clone() }),
        GameEvent::FaceDanced(_) => ActionOptions::FaceDanced(face_danced_options(game, p, dj.clone())),
        GameEvent::FaceDancerReplaced(_) => ActionOptions::FaceDancerReplaced(BattleHeroesOptions {
            heroes: FaceDancerReplaced::valid_face_dancers(game, p).map(|h| form_hero(h, d)).collect(), draft: dj.clone() }),
        GameEvent::FaceDancerRevealed(_) => ActionOptions::FaceDancerRevealed(BattleMayCallOptions { may_call: FaceDanced::may_call_face_dancer(game, p), draft: dj.clone() }),
        GameEvent::HMSAdvantageChosen(_) => ActionOptions::HMSAdvantageChosen(BattleChoicesOptions {
            choices: HMSAdvantageChosen::valid_advantages(game, p).into_iter().map(|a| enum_choice(a, d)).collect(), draft: dj.clone() }),
        GameEvent::JuicePlayed(_) => ActionOptions::JuicePlayed(BattleChoicesOptions {
            choices: JuicePlayed::valid_types(game, p).into_iter().map(|t| enum_choice(t, d)).collect(), draft: dj.clone() }),
        GameEvent::LoserConcluded(_) => ActionOptions::LoserConcluded(loser_concluded_options(game, p, dj.clone())),
        GameEvent::Planetology(_) => ActionOptions::Planetology(BattleButtonsOptions { draft: dj.clone() }),
        GameEvent::Prescience(_) => ActionOptions::Prescience(PrescienceOptions {
            aspects: Prescience::valid_aspects(game, p).into_iter().map(|a| enum_choice(a, d)).collect(),
            voice_note: game.current_battle.as_ref().is_some_and(|b| b.is_involved_faction(game, Faction::BeneGesserit)), draft: dj.clone() }),
        GameEvent::Retreat(_) => ActionOptions::Retreat(RetreatOptions {
            targets: Retreat::valid_targets(game, p).map(|l| l.id()).collect(),
            max_forces: Retreat::max_forces(game, p),
            max_special_forces: Retreat::max_special_forces(game, p),
            max_total_forces: Retreat::max_total_forces(game, p), draft: dj.clone() }),
        GameEvent::RockWasMelted(_) => ActionOptions::RockWasMelted(BattleButtonsOptions { draft: dj.clone() }),
        GameEvent::TakeLosses(_) => {
            let loss = TakeLosses::losses_to_take(game);
            let useless = if faction == Faction::Choam && TakeLosses::can_prevent_losses(game, p) { TakeLosses::valid_useless_card_to_prevent_losses(game, p) } else { None };
            ActionOptions::TakeLosses(TakeLossesOptions {
                amount: loss.amount,
                location: loss.location.map(|l| l.id()),
                max_forces: TakeLosses::valid_max_force_amount(game, p),
                max_special_forces: TakeLosses::valid_max_special_force_amount(game, p),
                useless_card: useless.map(|c| c.id()),
                useless_label: (faction == Faction::Choam && TakeLosses::can_prevent_losses(game, p))
                    .then(|| format1(d, "Use ", useless.map(ExpressionElement::from).unwrap_or(ExpressionElement::Null), " to prevent losses")), draft: dj.clone() })
        }
        GameEvent::ThoughtAnswered(_) => ActionOptions::ThoughtAnswered(BattleCardsOptions { cards: ThoughtAnswered::valid_cards(game, p).iter().map(|c| c.id()).collect(), draft: dj.clone() }),
        GameEvent::Thought(_) => ActionOptions::Thought(BattleCardsOptions { cards: Thought::valid_cards(game).map(|c| c.id()).collect(), draft: dj.clone() }),
        GameEvent::TreacheryCalled(_) => ActionOptions::TreacheryCalled(BattleMayCallOptions { may_call: TreacheryCalled::may_call_treachery(game, p), draft: dj.clone() }),
        GameEvent::Voice(_) => ActionOptions::Voice(VoiceOptions {
            types: voice_types(game, &Some(d)),
            prescience_note: game.current_battle.as_ref().is_some_and(|b| b.is_involved_faction(game, Faction::Atreides)), draft: dj.clone() }),
        _ => return None,
    };
    Some((Some(options), validate(game, draft.clone(), d)))
}

// ------------------------------------------------------------------------------------------
// Helpers.
// ------------------------------------------------------------------------------------------

fn d_none<'a>() -> Option<&'a dyn Describer> {
    None
}

fn enum_choice<T: NetEnum + Into<ExpressionElement>>(v: T, d: &dyn Describer) -> BattleFormEnum {
    BattleFormEnum { value: v.value(), name: T::INFO.format(v.value()), label: d.describe(&v.into()) }
}

fn form_hero(h: Hero, d: &dyn Describer) -> BattleFormHero {
    BattleFormHero { id: h.id(), hero: hero_ref(h), label: d.describe(&h.into()) }
}

/// `Skin.Format("<before>{0}<after>", x)`: the label of `x` between two texts.
fn format1(d: &dyn Describer, before: &str, x: ExpressionElement, after: &str) -> String {
    format!("{before}{}{after}", d.describe(&x))
}

fn loser(game: &Game) -> Option<PlayerIndex> {
    game.get_player(game.battle_loser)
}

/// `Voice.ValidTypes(Game).OrderBy(t => Name(t))`. Without a describer (the default draft's
/// first type), the default skin names them.
fn voice_types(game: &Game, d: &Option<&dyn Describer>) -> Vec<BattleFormEnum> {
    let fallback;
    let d: &dyn Describer = match d {
        Some(d) => *d,
        None => {
            fallback = crate::describe::default_describer();
            &*fallback
        }
    };
    let mut types: Vec<BattleFormEnum> = Voice::valid_types(game).into_iter().map(|t| enum_choice(t, d)).collect();
    // OrderBy is stable; the client runs with InvariantGlobalization, so strings compare ordinally.
    types.sort_by(|a, b| ordinal(&a.label, &b.label));
    types
}

/// .NET ordinal string comparison (UTF-16 code units).
fn ordinal(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// The retreat form's starting inputs: everything that may retreat, trimmed to the total
/// allowed, special forces first.
fn retreat_defaults(game: &Game, p: PlayerIndex) -> (i32, i32) {
    let mut forces = Retreat::max_forces(game, p);
    let mut special = Retreat::max_special_forces(game, p);
    let max = Retreat::max_total_forces(game, p);
    while forces + special > max {
        if special > 0 {
            special -= 1;
        } else {
            forces -= 1;
        }
    }
    (forces, special)
}

fn face_danced_options(game: &Game, p: PlayerIndex, draft: serde_json::Value) -> FaceDancedOptions {
    let pl = game.player(p);
    let at = |l: Location| FaceDancedSource { location: l.id(), forces: pl.forces_in(Some(l)), special_forces: pl.special_forces_in(Some(l)) };
    FaceDancedOptions {
        maximum_forces: FaceDanced::maximum_number_of_forces(game, p),
        forces_in_reserve: pl.forces_in_reserve(),
        territory: game.current_battle.as_ref().and_then(|b| b.territory(game)).map_or(-1, |t| t.id()),
        sources: FaceDanced::valid_source_locations(game, p).map(at).collect(),
        targets: FaceDanced::valid_target_locations(game).map(at).collect(),
        draft,
    }
}

fn loser_concluded_options(game: &Game, p: PlayerIndex, draft: serde_json::Value) -> LoserConcludedOptions {
    let may_use_karma = LoserConcluded::may_use_karma_to_force_kept_cards(game, p);
    LoserConcludedOptions {
        cards_to_keep: LoserConcluded::cards_loser_may_keep(game).iter().map(|c| c.id()).collect(),
        assassination: LoserConcluded::can_assassinate(game, p).then(|| {
            let target = LoserConcluded::target_of_assassination(game, p);
            let winner = game.get_player(game.battle_winner);
            let reward = match (winner, target) {
                (Some(w), Some(t)) => LoserConcluded::assassination_reward(game, Some(w), t),
                _ => None,
            };
            LoserConcludedAssassination { target: target.map_or(-1, |t| t.id()), reward }
        }),
        may_use_karma,
        cards_kept_or_discarded: if may_use_karma { LoserConcluded::cards_that_may_be_kept_or_discarded(game).iter().map(|c| c.id()).collect() } else { vec![] },
        draft,
    }
}

/// The faction's own plan in the current battle.
fn my_plan(game: &Game, faction: Faction) -> Option<Rc<Battle>> {
    let agg = game.aggressor_plan.as_ref()?;
    if agg.initiator() == faction {
        Some(agg.clone())
    } else {
        game.defender_plan.clone()
    }
}

fn battle_concluded_options(game: &Game, p: PlayerIndex, d: &dyn Describer, draft: serde_json::Value) -> BattleConcludedOptions {
    let pl = game.player(p);
    let faction = pl.faction;
    let winner = game.battle_winner == Some(faction);
    let supplier = Battle::determine_force_supplier(game, p);
    let plan = my_plan(game, faction);
    let in_hand = |c: Option<TreacheryCard>| c.filter(|c| winner && pl.treachery_cards.contains(c)).map(|c| c.id());
    let may_discard = winner && BattleConcluded::may_choose_to_discard_cards(game);
    let mercenary = plan.as_ref().and_then(|b| match b.hero() {
        Some(Hero::TreacheryCard(c)) => Some(c),
        _ => None,
    });
    BattleConcludedOptions {
        winner,
        force_supplier: game.player(supplier).faction.to_string(),
        max_replacement_forces: if winner { BattleConcluded::valid_max_replacement_force_amount(game, supplier) } else { 0 },
        discardable_mercenary: if may_discard { in_hand(mercenary) } else { None },
        discardable_weapon: if may_discard { in_hand(plan.as_ref().and_then(|b| b.weapon())) } else { None },
        discardable_defense: if may_discard { in_hand(plan.as_ref().and_then(|b| b.defense())) } else { None },
        capture: (winner && BattleConcluded::may_capture_or_kill(game, Some(p))).then(|| BattleConcludedCapture {
            victim: game.harkonnen_victim.map_or(-1, |l| l.id()),
            decisions: BattleConcluded::valid_capture_decisions(game).into_iter().map(|c| enum_choice(c, d)).collect(),
        }),
        tech_tokens: if winner { loser(game).map(|l| game.player(l).tech_tokens.iter().map(|t| enum_choice(*t, d)).collect()).unwrap_or_default() } else { vec![] },
        decipherer_traitors: if winner { game.traitors_decipherer_can_look_at.iter().map(|h| form_hero(*h, d)).collect() } else { vec![] },
        traitors_to_replace: (winner && !game.traitors_decipherer_can_look_at.is_empty() && game.decipherer_may_replace_traitor)
            .then(|| BattleConcluded::valid_traitors_to_replace(game, p).map(|h| form_hero(h, d)).collect()),
        add_extra_force_in: (winner && BattleConcluded::may_add_extra_force(game, p))
            .then(|| game.current_battle.as_ref().and_then(|b| b.territory(game)).map_or(-1, |t| t.id())),
        draft,
    }
}

/// The parameters a `ClairVoyancePlayed` question may take (the form's select values), and the
/// default ones for a question: each select takes its first value.
fn clairvoyance_lists(game: &Game, d: &dyn Describer) -> (Vec<BattleFormParam>, Vec<BattleFormParam>, Vec<BattleFormParam>, Vec<BattleFormParam>, Vec<BattleFormParam>) {
    let card_types = Voice::valid_types(game).into_iter().map(|t| BattleFormParam { value: t.to_string(), label: d.describe(&t.into()) }).collect();
    // Values: TraitorsInPlay ordered by name; labels: the names ordered (the same order).
    let mut leaders: Vec<(String, i32)> = game.traitors_in_play().into_iter().map(|h| (d.describe(&h.into()), h.id())).collect();
    leaders.sort_by(|a, b| ordinal(&a.0, &b.0));
    let leaders = leaders.into_iter().map(|(label, id)| BattleFormParam { value: id.to_string(), label }).collect();
    let factions = game.players.iter().map(|p| BattleFormParam { value: p.faction.to_string(), label: d.describe(&p.faction.into()) }).collect();
    let turns = BeneGesseritPrediction::valid_turns(game).into_iter().map(|t| BattleFormParam { value: t.to_string(), label: t.to_string() }).collect();
    let mut territories: Vec<(bool, String, i32)> = game.map.territories(true).map(|t| (t.is_stronghold(), d.describe(&t.into()), t.id())).collect();
    territories.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| ordinal(&a.1, &b.1)));
    let territories = territories.into_iter().map(|(_, label, id)| BattleFormParam { value: id.to_string(), label }).collect();
    (card_types, leaders, factions, turns, territories)
}

/// The dials of `DialOfMoreThanXInBattle`: 0 to 25 in steps of a half, as `float.ToString()`.
fn clairvoyance_dials() -> Vec<String> {
    let mut out = vec![];
    let mut current = 0.0f32;
    while current <= 25.0 {
        out.push(dotnet_compat::fmt::f32_to_string(current));
        current += 0.5;
    }
    out
}

fn clairvoyance_defaults(game: &Game, q: ClairvoyanceQuestion, d: Option<&dyn Describer>) -> (Option<String>, Option<String>) {
    let fallback;
    let d: &dyn Describer = match d {
        Some(d) => d,
        None => {
            fallback = crate::describe::default_describer();
            &*fallback
        }
    };
    let (card_types, leaders, factions, turns, territories) = clairvoyance_lists(game, d);
    let first = |l: &[BattleFormParam]| l.first().map(|p| p.value.clone());
    use ClairvoyanceQuestion as Q;
    match q {
        Q::CardTypeInBattle | Q::CardTypeAsWeaponInBattle | Q::CardTypeAsDefenseInBattle | Q::HasCardTypeInHand => (first(&card_types), None),
        Q::LeaderAsFacedancer | Q::LeaderAsTraitor | Q::LeaderInBattle => (first(&leaders), None),
        Q::Prediction => (first(&factions), first(&turns)),
        Q::DialOfMoreThanXInBattle => (clairvoyance_dials().first().cloned(), None),
        Q::WillAttackX => (first(&territories), None),
        _ => (None, None),
    }
}

fn clairvoyance_options(game: &Game, p: PlayerIndex, target: Faction, d: &dyn Describer, draft: serde_json::Value) -> ClairVoyancePlayedOptions {
    let (card_types, leaders, factions, turns, territories) = clairvoyance_lists(game, d);
    ClairVoyancePlayedOptions {
        targets: ClairVoyancePlayed::valid_targets(game, p).map(|f| enum_choice(f, d)).collect(),
        questions: ClairVoyancePlayed::valid_questions(game, target)
            .map(|q| {
                let label = d.describe(&ClairVoyancePlayed::express(q, ExpressionElement::Null, ExpressionElement::Null).into());
                BattleFormEnum { value: q.value(), name: q.to_string(), label }
            })
            .collect(),
        card_types,
        leaders,
        factions,
        turns,
        territories,
        draft,
    }
}

// ------------------------------------------------------------------------------------------
// The battle plan.
// ------------------------------------------------------------------------------------------

/// The battle plan form's starting inputs: the previous plan when one is valid and no plan is in
/// yet, the token rows trimmed to the forces there, the first leader when one is required.
fn battle_default(game: &Game, p: PlayerIndex, faction: Faction) -> GameEvent {
    // ConfirmedResult sets every member; a fresh event's ids are 0, which are real objects.
    let mut e = Battle::new(game, faction);
    e.set_hero(None);
    e.set_weapon(None);
    e.set_defense(None);
    if let Some(battle) = game.current_battle.as_ref() {
        if battle.plan_of(game, Some(p)).is_none() {
            if let Some(prev) = battle.previous_plan_of(game, faction) {
                if prev.validate(game).is_none() {
                    e.forces = prev.forces;
                    e.forces_at_half_strength = prev.forces_at_half_strength;
                    e.special_forces = prev.special_forces;
                    e.special_forces_at_half_strength = prev.special_forces_at_half_strength;
                    e.ally_contribution_amount = prev.ally_contribution_amount;
                    e.set_hero(prev.hero());
                    e.messiah = prev.messiah;
                    e.set_defense(prev.defense());
                    e.set_weapon(prev.weapon());
                    e.banker_bonus = prev.banker_bonus;
                }
            }
        }
    }
    let supplier = Battle::determine_force_supplier(game, p);
    // Row(full + half) resized to the tokens there: full first, then half.
    let trim = |full: i32, half: i32, len: i32| {
        let f = full.min(len);
        (f, half.min(len - f))
    };
    (e.forces, e.forces_at_half_strength) = trim(e.forces, e.forces_at_half_strength, Battle::max_forces(game, supplier, false));
    (e.special_forces, e.special_forces_at_half_strength) = trim(e.special_forces, e.special_forces_at_half_strength, Battle::max_forces(game, supplier, true));
    normalize_battle(game, p, &mut e);
    event(e)
}

/// The adjustments the battle plan form makes to its inputs, in order.
fn normalize_battle(game: &Game, p: PlayerIndex, e: &mut Battle) {
    let pl = game.player(p);
    let territory = game.current_battle.as_ref().and_then(|b| b.territory(game));
    // The spice the ally pays, between what the player can't pay and what the ally can.
    let cost = Battle::cost_game_player_int32_int32_int32(game, p, e.forces, e.special_forces, &mut 0);
    let ally_cap = ally_cap(game, p);
    let ally_min = 0.max(cost - pl.resources);
    let ally_max = cost.min(ally_cap);
    e.ally_contribution_amount = if ally_max < ally_min { ally_max } else { e.ally_contribution_amount.clamp(ally_min, ally_max) };

    let heroes = Battle::valid_battle_heroes(game, p);
    let leader_required = !game.applicable(Rule::BattleWithoutLeader);
    if e.hero().is_some_and(|h| !heroes.contains(&h)) {
        e.set_hero(None);
    }
    if e.hero().is_none() && leader_required && !heroes.is_empty() {
        e.set_hero(Some(heroes[0]));
    }
    let weapons = Battle::valid_weapons(game, p, e.defense(), e.hero(), territory, false);
    if e.weapon().is_some_and(|w| !weapons.contains(&Some(w))) {
        e.set_weapon(None);
    }
    let defenses = Battle::valid_defenses(game, p, e.weapon(), territory, false);
    if e.defense().is_some_and(|x| !defenses.contains(&Some(x))) {
        e.set_defense(None);
    }
    if !messiah_available(game, p) {
        e.messiah = false;
    }
    let max_banker = Battle::max_banker_boost(game, p, e.hero());
    // The spice picker clamps into [0, max] when it is drawn.
    e.banker_bonus = if max_banker <= 0 { 0 } else { e.banker_bonus.clamp(0, max_banker) };
}

fn messiah_available(game: &Game, p: PlayerIndex) -> bool {
    game.player(p).messiah_available(game) && !game.prevented(FactionAdvantage::AtreidesUseMessiah)
}

/// How much the ally may pay: only a CHOAM ally helps pay.
fn ally_cap(game: &Game, p: PlayerIndex) -> i32 {
    if game.player(p).ally == Faction::Choam {
        game.resources_your_ally_can_pay(p)
    } else {
        0
    }
}

/// Whether forces may be committed at half strength, which only applies when they must be paid
/// for.
fn half_allowed(game: &Game, p: PlayerIndex, special: bool) -> bool {
    if special {
        Battle::must_pay_for_special_forces_in_battle(game, p) && Battle::special_force_cost(game, p) > 0
    } else {
        Battle::must_pay_for_any_forces_in_battle(game, p) && Battle::normal_force_cost(game, p) > 0
    }
}

/// Every affordable way to commit the tokens.
fn mixes(game: &Game, p: PlayerIndex, opponent: Faction, normal: i32, special: i32) -> Vec<(i32, i32, i32, i32, f32, i32)> {
    let half_s = half_allowed(game, p, true);
    let half_n = half_allowed(game, p, false);
    let available = game.player(p).resources + ally_cap(game, p);
    let mut result = vec![];
    for fs in 0..=special {
        for fn_ in 0..=normal {
            let cost = Battle::cost_game_player_int32_int32_int32(game, p, fn_, fs, &mut 0);
            if cost > available {
                continue;
            }
            for hs in 0..=(if half_s { special - fs } else { 0 }) {
                for hn in 0..=(if half_n { normal - fn_ } else { 0 }) {
                    let dial = Battle::dial_game_player_faction_int32_int32_int32_int32(game, p, opponent, fn_, fs, hn, hs);
                    result.push((fs, hs, fn_, hn, dial, cost));
                }
            }
        }
    }
    result
}

fn force_group(game: &Game, p: PlayerIndex, territory: Option<Territory>, is_opponent: bool) -> BattleForceGroup {
    let pl = game.player(p);
    BattleForceGroup {
        faction: pl.faction.to_string(),
        forces: pl.forces_in_territory(territory),
        special_forces: pl.special_forces_in_territory(territory),
        hide_special_count: is_opponent && pl.faction == Faction::Richese,
    }
}

fn side(game: &Game, p: PlayerIndex, aggressor: Faction, groups: Vec<BattleForceGroup>) -> BattleSide {
    let pl = game.player(p);
    BattleSide {
        faction: pl.faction.to_string(),
        aggressor: aggressor == pl.faction,
        locations: pl.forces_on_planet().keys().map(|l| l.id()).collect(),
        groups,
    }
}

/// The note on the battle plan form saying what of `plan` prescience shows.
fn seen_note(game: &Game, faction: Faction, aspect: PrescienceAspect, plan: &Battle, show_everything: bool, d: &dyn Describer) -> BattleNote {
    let shown: Option<ExpressionElement> = match aspect {
        PrescienceAspect::Defense => plan.defense().filter(|c| show_everything || c.is_defense()).map(Into::into),
        PrescienceAspect::Weapon => plan.weapon().filter(|c| show_everything || c.is_weapon()).map(Into::into),
        PrescienceAspect::Leader => plan.hero().map(Into::into),
        _ => None,
    };
    BattleNote::Seen {
        aspect: d.describe(&aspect.into()),
        dial: (aspect == PrescienceAspect::Dial).then(|| dotnet_compat::fmt::f32_to_string(plan.dial(game, faction))),
        shown: shown.map(|el| {
            let mut out = vec![];
            expr::element_into(&el, d, &mut out);
            out
        }),
    }
}

fn battle_notes(game: &Game, p: PlayerIndex, opponent: PlayerIndex, d: &dyn Describer) -> Vec<BattleNote> {
    let pl = game.player(p);
    let faction = pl.faction;
    let battle = game.current_battle.as_ref().expect("a battle plan needs a battle");
    let ecaz_side = faction == Faction::Ecaz || pl.ally == Faction::Ecaz;
    let mut notes = vec![];
    if game.current_ecaz_battle_contribution > 0 && !ecaz_side {
        notes.push(BattleNote::EcazOpponent { amount: game.current_ecaz_battle_contribution });
    }
    if Battle::valid_battle_heroes(game, opponent).is_empty() {
        notes.push(BattleNote::OpponentNoLeaders);
    }
    if let Some(voice) = game.current_voice.as_ref() {
        notes.push(BattleNote::Voice {
            mine: faction == Faction::BeneGesserit || pl.ally == Faction::BeneGesserit,
            must: voice.must,
            card_type: d.describe(&voice.type_.into()),
        });
    }
    let used_by_my_side = |initiator: Option<Faction>| initiator.is_some_and(|i| i == faction || i == pl.ally);
    let prescience_by_me = used_by_my_side(game.current_prescience.as_ref().map(|e| e.initiator()));
    let nexus_by_me = used_by_my_side(game.current_nexus_prescience.as_ref().map(|e| e.initiator()));
    let atreides_side = faction == Faction::Atreides || pl.ally == Faction::Atreides;
    if atreides_side || prescience_by_me || nexus_by_me {
        let opponent_plan = if battle.aggressor(game) == faction { game.defender_plan.clone() } else { game.aggressor_plan.clone() };
        if game.atreides_karma && atreides_side {
            match &opponent_plan {
                Some(plan) => {
                    for aspect in [PrescienceAspect::Dial, PrescienceAspect::Weapon, PrescienceAspect::Defense, PrescienceAspect::Leader] {
                        notes.push(seen_note(game, faction, aspect, plan, true, d));
                    }
                }
                None => notes.push(BattleNote::Later { aspect: None }),
            }
        } else {
            if prescience_by_me {
                let aspect = game.current_prescience.as_ref().unwrap().aspect;
                notes.push(match &opponent_plan {
                    Some(plan) => seen_note(game, faction, aspect, plan, false, d),
                    None => BattleNote::Later { aspect: Some(d.describe(&aspect.into())) },
                });
            }
            if nexus_by_me {
                let aspect = game.current_nexus_prescience.as_ref().unwrap().atreides_prescience_aspect;
                notes.push(match &opponent_plan {
                    Some(plan) => seen_note(game, faction, aspect, plan, false, d),
                    None => BattleNote::Later { aspect: Some(d.describe(&aspect.into())) },
                });
            }
        }
    }
    if game.current_ecaz_battle_contribution > 0 && ecaz_side {
        notes.push(BattleNote::EcazMine { amount: game.current_ecaz_battle_contribution });
    }
    notes
}

/// Sorts cards by `Skin.Describe` (`OrderBy(Name)`), ordinally and stably.
fn by_name(cards: Vec<Option<TreacheryCard>>, d: &dyn Describer) -> Vec<i32> {
    let mut named: Vec<(String, i32)> = cards.into_iter().flatten().map(|c| (d.describe(&c.into()), c.id())).collect();
    named.sort_by(|a, b| ordinal(&a.0, &b.0));
    named.into_iter().map(|(_, id)| id).collect()
}

fn battle_options(game: &Game, p: PlayerIndex, draft: &Battle, d: &dyn Describer) -> (BattleOptions, Battle) {
    let pl = game.player(p);
    let faction = pl.faction;
    let battle = game.current_battle.as_ref().expect("a battle plan needs a battle");
    let territory = battle.territory(game);
    let opponent = battle.opponent_of(game, p).expect("a battle has an opponent");
    let supplier = Battle::determine_force_supplier(game, p);
    let aggressor = battle.aggressor(game);

    let mut normalized = draft.clone();
    normalize_battle(game, p, &mut normalized);
    let weapons = by_name(Battle::valid_weapons(game, p, normalized.defense(), normalized.hero(), territory, false), d);
    let defenses = by_name(Battle::valid_defenses(game, p, normalized.weapon(), territory, false), d);

    let mut opponent_groups = vec![force_group(game, opponent, territory, true)];
    if game.player(opponent).has_ally() {
        if let Some(ally) = game.player(opponent).allied_player(game) {
            opponent_groups.push(force_group(game, ally, territory, true));
        }
    }

    let normal_tokens = Battle::max_forces(game, supplier, false);
    let special_tokens = Battle::max_forces(game, supplier, true);
    let hero = normalized.hero();
    let leader = hero.map(|h| {
        let mut skill = LeaderSkill::None;
        let bonus = Battle::determine_skill_bonus_game_player_i_hero_treachery_card_treachery_card_int32_leader_skill(
            game,
            p,
            Some(h),
            normalized.weapon(),
            normalized.defense(),
            normalized.banker_bonus,
            &mut skill,
        );
        BattleLeaderStrength { value: h.value_in_combat_against(None), skill_bonus: bonus, skill_label: d.describe(&skill.into()) }
    });
    let banker_label = format!(
        "Boost {} {} with {}?",
        d.describe(&LeaderSkill::Banker.into()),
        d.describe(&hero.map(ExpressionElement::from).unwrap_or(ExpressionElement::Null)),
        d.describe(&Concept::Resource.into())
    );

    let options = BattleOptions {
        territory: territory.map_or(-1, |t| t.id()),
        territory_label: territory.map(|t| d.describe(&t.into())).unwrap_or_default(),
        territory_locations: territory.map(|t| t.locations().iter().map(|l| l.id()).collect()).unwrap_or_default(),
        you: side(game, p, aggressor, vec![force_group(game, supplier, territory, false)]),
        opponent: side(game, opponent, aggressor, opponent_groups),
        notes: battle_notes(game, p, opponent, d),
        supplier: game.player(supplier).faction.to_string(),
        normal_tokens,
        special_tokens,
        half_normal: half_allowed(game, p, false),
        half_special: half_allowed(game, p, true),
        must_pay: Battle::must_pay_for_any_forces_in_battle(game, p),
        resources: pl.resources,
        ally_cap: ally_cap(game, p),
        ally: pl.ally.to_string(),
        mixes: mixes(game, p, game.player(opponent).faction, normal_tokens, special_tokens),
        heroes: Battle::valid_battle_heroes(game, p).into_iter().map(|h| form_hero(h, d)).collect(),
        leader_required: !game.applicable(Rule::BattleWithoutLeader),
        weapons,
        defenses,
        messiah_available: messiah_available(game, p),
        max_banker_boost: Battle::max_banker_boost(game, p, hero),
        banker_label,
        leader,
        draft: draft_json(&event(normalized.clone())),
    };
    let _ = faction;
    (options, normalized)
}
