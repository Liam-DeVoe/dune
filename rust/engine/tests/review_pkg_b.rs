//! Package B review (Player.cs, PlayerSequence.cs, Battalion.cs, Game_Rules.cs,
//! Game_Events.cs). See docs/reviews/pkg-b.md.
//!
//! The expected lines come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-b ../tests/review_pkg_b/probes.txt
//!
//! `player_force_operations_match_csharp` applies the same pseudo-random sequence of
//! force operations to a game in both engines (identical xorshift32 generator) and
//! compares the player state after every operation. `determine_first_player_matches_csharp`
//! checks `PlayerSequence.DetermineFirstPlayer` over storm sectors and player counts.

use treachery_engine::compat::cs;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;

const GOLDEN: &str = include_str!("review_pkg_b/probes.txt");

struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u32) as usize
    }
    fn bool(&mut self) -> bool {
        self.below(2) == 1
    }
}

const FACTIONS: [Faction; 6] = [Faction::Emperor, Faction::BeneGesserit, Faction::Richese, Faction::Fremen, Faction::Ixian, Faction::Choam];

fn new_game(version: i32) -> Game {
    let mut g = Game::new_int32_participation(version, &treachery_engine::compat::cs::Shared::new(Participation::new()));
    g.rules.push(Rule::Homeworlds);
    g.rules.push(Rule::EmperorSpecialForces);
    g.rules.push(Rule::FremenSpecialForces);
    for (i, f) in FACTIONS.iter().enumerate() {
        let mut p = Player::new(&g, *f);
        p.seat = i as i32;
        g.players.push(p);
    }
    for i in 0..g.players.len() {
        let f = g.players[i].faction;
        let hws: Vec<Homeworld> = g.map.homeworlds().filter(|w| w.faction() == f).collect();
        for hw in hws {
            let normal = if hw.is_home_of_normal_forces() { 10 } else { 0 };
            let special = if hw.is_home_of_special_forces() { 4 } else { 0 };
            let me = g.player_at(i);
            Player::initialize_homeworld(&mut g, me, hw, normal, special);
        }
    }
    g
}

fn location_pool(g: &Game) -> Vec<Location> {
    g.map.locations(true).filter(|l| l.id() % 9 == 0 || l.as_homeworld().is_some()).collect()
}

fn territory_pool(g: &Game) -> Vec<Territory> {
    let mut v: Vec<Territory> = Vec::new();
    for l in location_pool(g) {
        if !v.contains(&l.territory()) {
            v.push(l.territory());
        }
    }
    v
}

fn b(x: bool) -> &'static str {
    if x { "True" } else { "False" }
}

fn join<T: ToString>(it: impl Iterator<Item = T>) -> String {
    it.map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn state(g: &Game, me: PlayerIndex) -> String {
    let p = g.player(me);
    let mut s = format!("{} [", p.faction);
    s += &join(p.forces_in_locations.iter().map(|(k, v)| format!("{}:{}/{}", k.id(), v.amount_of_forces, v.amount_of_special_forces)));
    s += &format!("] k={}/{}/{}", p.forces_killed, p.special_forces_killed, p.total_forces_killed_in_battle);
    s += &format!(" res={}/{}", p.forces_in_reserve(), p.special_forces_in_reserve());
    s += &format!(" hi={} lo={}", b(p.has_high_threshold_0(g)), b(p.has_low_threshold_0(g)));
    for hw in &p.home_worlds {
        s += &format!(
            " w{}={}/{}/{}",
            hw.world().0,
            b(p.has_high_threshold(g, hw.world())),
            b(p.has_low_threshold(g, hw.world())),
            p.get_homeworld_battle_contribution_and_lasgun_shield_limit(g, hw.location().territory())
        );
    }
    s += &format!(" nf={}", b(p.no_field_is_active()));
    s += &format!(" occ={}", join(p.occupied_territories(g).iter().map(|t| t.id())));
    s += &format!(" twf={}", join(p.territories_with_forces(g).map(|t| t.id())));
    s += &format!(" planet={}", join(p.forces_on_planet().keys().map(|l| l.id())));
    s += " each=";
    for o in 0..g.players.len() {
        s += if p.have_forces_on_each_others_home_worlds(g, g.player_at(o)) { "1" } else { "0" };
    }
    s
}

fn rust_lines() -> Vec<String> {
    let mut out = Vec::new();
    for version in [Game::LATEST_VERSION] {
        for seed in 1u32..=20 {
            let mut g = new_game(version);
            let locs = location_pool(&g);
            let terrs = territory_pool(&g);
            let hws: Vec<Homeworld> = g.map.homeworlds().collect();
            if seed == 1 {
                out.push(format!("pool {} {} | {}", version, join(locs.iter().map(|l| l.id())), join(terrs.iter().map(|t| t.id()))));
            }
            let mut r = Rng::new(seed.wrapping_mul(2654435761));
            for step in 0..120 {
                let me = g.player_at(r.below(g.players.len()));
                let op = r.below(25);
                let l = locs[r.below(locs.len())];
                let l2 = locs[r.below(locs.len())];
                let t = terrs[r.below(terrs.len())];
                let hw = hws[r.below(hws.len())];
                let n = r.below(9) as i32 - 2;
                let m = r.below(9) as i32 - 2;
                let bb = r.bool();
                let res = match catch(|| {
                    let g = &mut g;
                    let mut ret: Option<i32> = None;
                    match op {
                        0 => Player::add_forces(g, me, l, n, bb),
                        1 => Player::add_special_forces(g, me, l, n, bb),
                        2 => Player::add_forces_to_reserves(g, me, n),
                        3 => Player::add_special_forces_to_reserves(g, me, n),
                        4 => Player::remove_forces(g, me, l, n),
                        5 => Player::remove_special_forces(g, me, l, n),
                        6 => Player::forces_to_reserves_location_int32(g, me, l, n),
                        7 => Player::special_forces_to_reserves(g, me, l, n),
                        8 => Player::forces_to_reserves_territory_int32_boolean(g, me, t, n, bb),
                        9 => Player::forces_to_reserves_territory(g, me, t),
                        10 => ret = Some(Player::kill_all_forces(g, me, l, bb)),
                        11 => Player::kill_all_forces_territory_boolean(g, me, t, bb),
                        12 => ret = Some(Player::kill_forces(g, me, l, n, m, bb)),
                        13 => {
                            let ib = r.bool();
                            Player::kill_forces_territory_int32_boolean_boolean(g, me, t, n, bb, ib)
                        }
                        14 => Player::revive_forces(g, me, n),
                        15 => Player::revive_special_forces(g, me, n),
                        16 => Player::ship_forces(g, me, l, n),
                        17 => Player::ship_forces_location_homeworld_int32(g, me, l, hw, n),
                        18 => Player::ship_special_forces(g, me, l, n),
                        19 => Player::ship_special_forces_location_homeworld_int32(g, me, l, hw, n),
                        20 => Player::ship_advisors(g, me, l, n),
                        21 => Player::move_forces(g, me, l, l2, n),
                        22 => Player::move_special_forces(g, me, l, l2, n),
                        23 => Player::flip_forces(g, me, l, bb),
                        24 => Player::flip_forces_territory_boolean(g, me, t, bb),
                        _ => unreachable!(),
                    }
                    ret
                }) {
                    Ok(Some(v)) => v.to_string(),
                    Ok(None) => "-".to_string(),
                    Err(()) => "exception".to_string(),
                };
                out.push(format!("{} {} {} op{} {} {}", version, seed, step, op, res, state(&g, me)));
            }
        }
    }

    for players in 1..=12 {
        for sector in -2..=19 {
            let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &treachery_engine::compat::cs::Shared::new(Participation::new()));
            g.settings.as_ref().unwrap().borrow_mut().number_of_players = players;
            for i in 0..players {
                if i % 3 == 1 {
                    continue;
                }
                let mut p = Player::new(&g, FACTIONS[(i as usize) % FACTIONS.len()]);
                p.seat = i;
                g.players.push(p);
            }
            g.sector_in_storm = sector;
            let res = match catch(|| PlayerSequence::determine_first_player(&g)) {
                Ok(fp) => g.player(fp).seat.to_string(),
                Err(()) => "exception".to_string(),
            };
            out.push(format!("first {} {} {}", players, sector, res));
        }
    }
    out
}

/// The golden lines write a C# exception as `exception:<Type>:<Message>`; the Rust lines write
/// any panic as `exception`. The exception's type and message aren't compared: everything from
/// `exception:` to the next state field (` fp=`) becomes `exception`.
fn without_exception_text(line: &str) -> String {
    match line.find("exception:") {
        Some(i) => {
            let rest = &line[i..];
            let end = rest.find(" fp=").unwrap_or(rest.len());
            format!("{}exception{}", &line[..i], &rest[end..])
        }
        None => line.to_string(),
    }
}

/// Runs `f`, catching any panic.
fn catch<R>(f: impl FnOnce() -> R) -> Result<R, ()> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cs::catch_panic(f))).ok().and_then(Result::ok).ok_or(())
}

fn compare(prefix_filter: impl Fn(&str) -> bool) {
    let expected: Vec<String> = GOLDEN.lines().filter(|l| prefix_filter(l)).map(without_exception_text).collect();
    let actual: Vec<String> = rust_lines().into_iter().filter(|l| prefix_filter(l)).collect();
    for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
        assert_eq!(e, a, "first difference at filtered line {i}");
    }
    assert_eq!(expected.len(), actual.len());
}

#[test]
fn player_force_operations_match_csharp() {
    compare(|l| !l.starts_with("first ") && !l.starts_with("seq "));
}

#[test]
fn determine_first_player_matches_csharp() {
    compare(|l| l.starts_with("first "));
}

fn seat(g: &Game, p: Option<PlayerIndex>) -> String {
    match p {
        Some(p) => g.player(p).seat.to_string(),
        None => "null".to_string(),
    }
}

fn try_s(f: impl FnOnce() -> String) -> String {
    catch(f).unwrap_or_else(|()| "exception".to_string())
}

fn tf(x: bool) -> String {
    b(x).to_string()
}

fn seq_state(g: &Game, s: &PlayerSequence) -> String {
    format!(
        "fp={} cf={} played={} round={} circ={} cur={} richese={} seq={}",
        seat(g, s.first_player),
        seat(g, s.current_player),
        s.played.iter().map(|p| seat(g, *p)).collect::<Vec<_>>().join(","),
        s.round,
        s.fully_circled,
        try_s(|| seat(g, s.current_player(g))),
        try_s(|| tf(s.has_passed_richese(g))),
        try_s(|| s
            .get_players_in_sequence(g)
            .map(|e| seat(g, e.player) + if e.has_turn { "*" } else { "" })
            .collect::<Vec<_>>()
            .join(",")),
    )
}

fn sequence_lines() -> Vec<String> {
    let mut out = Vec::new();
    let cards = TreacheryCardManager::items();
    for version in [Game::LATEST_VERSION] {
        for seed in 1u32..=150 {
            let mut r = Rng::new(seed.wrapping_mul(40503).wrapping_add(7));
            let mut g = Game::new_int32_participation(version, &treachery_engine::compat::cs::Shared::new(Participation::new()));
            let count = 1 + r.below(6);
            let np = (count + r.below(2)) as i32;
            g.settings.as_ref().unwrap().borrow_mut().number_of_players = np;
            let factions = [Faction::Atreides, Faction::Richese, Faction::Harkonnen, Faction::Emperor, Faction::Choam, Faction::BeneGesserit, Faction::Tleilaxu];
            for i in 0..count {
                let mut p = Player::new(&g, factions[(i + seed as usize) % factions.len()]);
                p.seat = if r.below(4) == 0 { r.below(count + 1) as i32 } else { i as i32 };
                let full = if r.below(3) == 0 { 8 } else { r.below(5) };
                for c in 0..full {
                    p.treachery_cards.push(cards[c]);
                }
                g.players.push(p);
            }
            g.sector_in_storm = r.below(18) as i32;
            let juice = r.below(4);
            if juice > 0 {
                let f = g.players[r.below(count)].faction;
                let mut j = JuicePlayed::new(&g, f);
                j.type_ = match juice {
                    1 => JuiceType::GoFirst,
                    2 => JuiceType::GoLast,
                    _ => JuiceType::Aggressor,
                };
                g.current_juice = Some(std::rc::Rc::new(j));
            }
            let skip = r.bool();
            let dir = if r.bool() { 1 } else { -1 };
            let begin = r.bool();
            let mut start_idx = r.below(count + 1);
            let nulls = start_idx == count;
            if nulls && !(begin && count >= 2 && !skip) {
                start_idx = 0;
            }
            let start = if start_idx == count { None } else { Some(g.player_at(start_idx)) };
            let kind = r.below(3);
            let head = format!(
                "seq {} {} kind={} skip={} dir={} begin={} start={}",
                version,
                seed,
                kind,
                b(skip),
                dir,
                b(begin),
                seat(&g, start)
            );
            let mut s: Option<PlayerSequence> = None;
            let ctor = try_s(|| {
                s = Some(match kind {
                    0 => PlayerSequence::new(&g, skip, dir, start, begin),
                    1 => PlayerSequence::new_game_boolean_int32(&g, skip, dir),
                    _ => PlayerSequence::new_game(&g),
                });
                "ok".to_string()
            });
            out.push(format!("{head} {ctor}"));
            let Some(mut s) = s else { continue };
            out.push(format!("{head} init {}", seq_state(&g, &s)));
            for step in 0..14 {
                let op = r.below(5);
                if op == 4 && !g.players.is_empty() {
                    let i = r.below(count);
                    let p = &mut g.players[i];
                    if !p.treachery_cards.is_empty() {
                        p.treachery_cards.remove(0);
                    } else {
                        p.treachery_cards.push(cards[0]);
                    }
                }
                let res = try_s(|| {
                    match op {
                        0 | 1 => s.next_player(&g),
                        2 => s.next_round(&g),
                        3 => s.check_current_player(&g),
                        _ => {}
                    }
                    "ok".to_string()
                });
                out.push(format!("{head} {step} op{op} {res} {}", seq_state(&g, &s)));
            }
            let a = g.player_at(r.below(count));
            let bb = g.player_at(r.below(count));
            out.push(format!("{head} isafter {}", try_s(|| tf(PlayerSequence::is_after(&g, a, bb)))));
        }
    }
    out
}

#[test]
fn player_sequence_matches_csharp() {
    let expected: Vec<String> = GOLDEN.lines().filter(|l| l.starts_with("seq ")).map(without_exception_text).collect();
    let actual = sequence_lines();
    for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
        assert_eq!(e, a, "first difference at sequence line {i}");
    }
    assert_eq!(expected.len(), actual.len());
}
