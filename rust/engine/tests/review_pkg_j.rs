//! Package J review (faction specials: Karma, Nexus, Moritani, Ecaz, Choam, clairvoyance):
//! tests that show where the Rust port behaves differently from C#. See
//! docs/reviews/pkg-j.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-j
//!
//! (`ReviewJ.cs`). Each test names the probe lines it checks. These tests are meant
//! to FAIL until the difference is fixed.
//!
//! Every test puts a null card into a hand. C# gets there with validation on:
//! `RicheseGaveCard.Validate` always returns null, and `_cardId = -1` makes
//! `Card` null, so `target.TreacheryCards.Add(Card)` appends a null to the ally's
//! hand (`Events/RicheseGaveCard.cs:42,61`). Rust ports that as
//! `push(card.or_null())` (`events/richese_gave_card.rs:69`).

use treachery_engine::canon;
use treachery_engine::cs;
use treachery_engine::enumerations::{Faction, MainPhase, Phase};
use treachery_engine::events::choam_discarded::ChoamDiscarded;
use treachery_engine::events::karma::Karma;
use treachery_engine::events::nexus_played::NexusPlayed;
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;
use treachery_engine::treachery_card::TreacheryCard;
use treachery_engine::treachery_card_manager::TreacheryCardManager;

// ReviewA.Establish with AllowedFactionsInPlay [Choam, Atreides, Richese] (same text as ReviewJ.cs).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[90,20,100],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Choam,Atreides,Richese","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

const KARAMA: i32 = 23; // probe: karama.id = 23
const USELESS: i32 = 28; // probe: useless.id = 28

fn new_game() -> Game {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &cs::Shared::new(p));
    let e: GameEvent = json::deserialize(ESTABLISH).expect("event json");
    e.initialize_game(&g);
    assert!(e.execute(&mut g, true, true).is_none(), "establish rejected");
    g
}

/// The value, or a failure that shows the Rust exception where C# returned normally.
fn ok<T>(r: Result<T, cs::CsException>, what: &str) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!("{what}; Rust threw {}: {}", e.type_name, e.message),
    }
}

fn card(id: i32) -> TreacheryCard {
    TreacheryCardManager::get(id).unwrap()
}

fn faction_index(g: &Game, f: Faction) -> usize {
    g.players.iter().position(|p| p.faction == f).unwrap()
}

/// `Karma.ValidKarmaCards` is a lazy `Where`, and every caller stops early: `Any()`
/// in `CanKarma`, `FirstOrDefault()` in the eleven Karama events. Rust collects the
/// whole hand first, so a null card after the first Karama throws.
///
/// Probe: `karma_null.can_karma = True`, `karma_null.first = 23`,
/// `karma_null.has_karma = True`.
#[test]
fn karma_valid_cards_stop_at_first_match() {
    let mut g = new_game();
    let p = g.player_at(0);
    g.players[0].treachery_cards = vec![card(KARAMA), TreacheryCard::NULL];

    let can = cs::catch(|| Karma::can_karma(&g, p));
    assert_eq!(ok(can, "C# CanKarma returns True"), true);

    let first = cs::catch(|| Karma::valid_karma_cards(&g, p).into_iter().next().map(|c| c.id()));
    assert_eq!(ok(first, "C# FirstOrDefault returns card 23"), Some(KARAMA));

    let has = cs::catch(|| g.player(p).has_karma(&g));
    assert_eq!(ok(has, "C# HasKarma returns True"), true);
}

/// `ChoamDiscarded.ValidCards(p).Contains(card)` and `.Any()` (validation and
/// `AddPlayerActions`), and `NexusPlayed.ValidChoamCards(p).Any()` (`IsApplicable`),
/// stop at the first match in C#.
///
/// Probe: `choam_discarded_null.contains = True`, `choam_discarded_null.any = True`,
/// `nexus_valid_choam_cards.any = True`.
#[test]
fn choam_valid_cards_stop_at_first_match() {
    let mut g = new_game();
    let b = faction_index(&g, Faction::Choam);
    let choam = g.player_at(b);
    g.players[b].treachery_cards = vec![card(USELESS), TreacheryCard::NULL];

    let contains = cs::catch(|| ChoamDiscarded::valid_cards(&g, choam).any(|c| c == card(USELESS)));
    assert_eq!(ok(contains, "C# returns True"), true);

    let any = cs::catch(|| NexusPlayed::valid_choam_cards(&g, choam).next().is_some());
    assert_eq!(ok(any, "C# returns True"), true);
}

/// `NexusPlayed` betrayal of Choam (`Events/NexusPlayed.cs:417-421`): with Choam's
/// hand `[null]`, `RandomOrDefault` returns null and `if (card != null)` skips the
/// discard. Rust's `random_or_default` returns `Some(TreacheryCard::NULL)`, which
/// `discard_treachery_card_boolean` treats as a card and then throws on.
///
/// Probe: `nexus_choam.applicable = True`, `nexus_choam.result = null`,
/// `nexus_choam.history = 2`, `nexus_choam.choam_hand = [null]`,
/// `nexus_choam.discard_pile = 0`, `nexus_choam.actor_nexus = None`.
#[test]
fn nexus_choam_betrayal_skips_null_card() {
    let mut g = new_game();
    let b = faction_index(&g, Faction::Choam);
    let a = g.players.iter().position(|p| p.faction != Faction::Choam).unwrap();
    assert_eq!(g.players[a].faction, Faction::Atreides); // probe: nexus_choam.actor = Atreides
    g.players[a].nexus = Faction::Choam;
    g.players[b].treachery_cards = vec![TreacheryCard::NULL];
    assert!(NexusPlayed::is_applicable(&g, g.player_at(a)));

    let text = format!(
        r#"{{"$type":"NexusPlayed","Faction":90,"Initiator":{},"Time":"2024-01-01T12:02:40+00:00"}}"#,
        Faction::Atreides.0
    );
    let e: GameEvent = json::deserialize(&text).expect("event json");
    e.initialize_game(&g);
    let r = e.execute(&mut g, true, true);
    assert!(r.is_none(), "C# accepts the event; Rust returned {:?}", r.map(|m| m.to_cs_string()));
    assert_eq!(g.history.len(), 2);
    assert_eq!(g.players[b].treachery_cards, vec![TreacheryCard::NULL]);
    assert_eq!(g.players[a].nexus, Faction::None);
}

fn exec(g: &mut Game, text: &str) -> Option<String> {
    let e: GameEvent = json::deserialize(text).expect("event json");
    e.initialize_game(g);
    e.execute(g, true, true).map(|m| m.to_cs_string())
}

fn hash(g: &Game) -> String {
    sha256_hex(&canon::dump_game(g))
}

/// The same difference reached with validation on for every event, as a fuzzed game
/// can reach it. Richese gives its ally Choam an unknown card (`RicheseGaveCard`,
/// `_cardId = -1`), so Choam holds `[null]`. Then Atreides, holding Choam's Nexus card,
/// betrays Choam. Choam's own applicable events throw on the null card in both
/// engines, but Atreides's do not.
///
/// The phase is set directly (Contemplate), as `ReviewJ.cs` does by reflection.
///
/// Probe: `richese_gave.result = null`, `richese_gave.choam_hand = [null]`,
/// `richese_gave.hash = 63ee...`, `richese_gave.nexus_result = null`,
/// `richese_gave.nexus_history = 3`, `richese_gave.nexus_choam_hand = [null]`,
/// `richese_gave.nexus_atreides_nexus = None`, `richese_gave.nexus_hash = 8dcd...`.
#[test]
fn richese_gave_null_card_then_nexus_choam_betrayal() {
    let mut g = new_game();
    let w = faction_index(&g, Faction::Richese);
    let b = faction_index(&g, Faction::Choam);
    let gr = faction_index(&g, Faction::Atreides);
    g.players[w].ally = Faction::Choam;
    g.players[b].ally = Faction::Richese;
    g.players[w].treachery_cards = vec![card(48)];
    g.players[b].treachery_cards = vec![];
    g.players[gr].nexus = Faction::Choam;
    g.current_main_phase = MainPhase::Contemplate;
    g.current_phase = Phase::Contemplate;

    let r = exec(&mut g, r#"{"$type":"RicheseGaveCard","_cardId":-1,"Initiator":100,"Time":"2024-01-01T12:02:40+00:00"}"#);
    assert_eq!(r, None);
    assert_eq!(g.players[b].treachery_cards, vec![TreacheryCard::NULL]);
    assert_eq!(hash(&g), "63ee120ad2c13884acfe9287b9172c4f991b5ea199b6954d63502bbb639ffff3", "state before the Nexus card");

    let r = exec(&mut g, r#"{"$type":"NexusPlayed","Faction":90,"Initiator":20,"Time":"2024-01-01T12:02:41+00:00"}"#);
    assert_eq!(r, None, "C# accepts the betrayal and skips the null card");
    assert_eq!(g.history.len(), 3);
    assert_eq!(g.players[b].treachery_cards, vec![TreacheryCard::NULL]);
    assert_eq!(g.players[gr].nexus, Faction::None);
    assert_eq!(hash(&g), "8dcdbe046cddad8e36eb78dd043c618d58e02026594173fab94f3bdf913a36cd");
}

// ---- SHA-256 (the engine crate has no hashing dependency; copied from review_pkg_i.rs) ----

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}
