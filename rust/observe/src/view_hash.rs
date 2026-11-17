//! The hashes of views in a trace's `views` lists.
//!
//! A game builds tens of thousands of views of about 80 KB each, so the hash is a good part of a
//! replay's time. Traces recorded before 2026-11-16 hold SHA-256 prefixes. Later ones hold
//! [`Fold64`] of the view's JSON, a much faster hash, and newer ones a [`Sections`] hash, which
//! hashes the parts that views share only once. The header says which (`viewHash`). A replay
//! hashes its views the way the trace's header says, so old traces still replay.
//!
//! Neither is meant to resist an attacker: a hash only stands for a view that the replay builds
//! again, and two different views get the same hash with a chance of about 2^-64.

use std::any::Any;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use treachery_view::schema::{BoardView, GameView, LogView, ReportLines};

/// How a trace hashes its views. Traces without `viewHash` in their header use SHA-256.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViewHash {
    /// The first 8 bytes of the SHA-256 of the view's JSON, as 16 hex digits.
    #[default]
    Sha256,
    /// [`Fold64`] of the view's JSON, as 16 hex digits.
    Fold64,
    /// [`Sections::hash`]: [`Fold64`] of the hashes of the view's fields, as 16 hex digits.
    Sections,
}

impl ViewHash {
    /// What the recorder writes.
    pub const LATEST: ViewHash = ViewHash::Sections;

    pub const ALL: [ViewHash; 3] = [ViewHash::Sha256, ViewHash::Fold64, ViewHash::Sections];

    pub fn name(self) -> &'static str {
        match self {
            ViewHash::Sha256 => "sha256",
            ViewHash::Fold64 => "fold64",
            ViewHash::Sections => "sections",
        }
    }

    pub fn parse(name: &str) -> Option<ViewHash> {
        ViewHash::ALL.into_iter().find(|h| h.name() == name)
    }

    /// A hasher of the view's JSON. Not for [`ViewHash::Sections`], which hashes the view
    /// itself.
    pub fn hasher(self) -> Hasher {
        match self {
            #[cfg(target_arch = "aarch64")]
            ViewHash::Sha256 if crate::sha256_arm::available() => Hasher::Sha256Arm(crate::sha256_arm::Sha256Arm::new()),
            ViewHash::Sha256 => Hasher::Sha256(Sha256::new()),
            ViewHash::Fold64 => Hasher::Fold64(Fold64::new()),
            ViewHash::Sections => panic!("a sections hash is made from the view, not from its JSON"),
        }
    }

    /// The hash of `bytes`, as written in a trace.
    pub fn of(self, bytes: &[u8]) -> String {
        let mut h = self.hasher();
        h.update(bytes);
        h.finish()
    }
}

/// A hash being computed. Serialize a view into it with `serde_json::to_writer`.
pub enum Hasher {
    Sha256(Sha256),
    /// SHA-256 with the CPU's SHA-2 instructions, several times faster.
    #[cfg(target_arch = "aarch64")]
    Sha256Arm(crate::sha256_arm::Sha256Arm),
    Fold64(Fold64),
}

impl Hasher {
    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            Hasher::Sha256(h) => Digest::update(h, bytes),
            #[cfg(target_arch = "aarch64")]
            Hasher::Sha256Arm(h) => h.update(bytes),
            Hasher::Fold64(h) => h.update(bytes),
        }
    }

    /// The hash as 16 hex digits.
    pub fn finish(self) -> String {
        match self {
            Hasher::Sha256(h) => hex8(&h.finalize()),
            #[cfg(target_arch = "aarch64")]
            Hasher::Sha256Arm(h) => hex8(&h.finalize()),
            Hasher::Fold64(h) => format!("{:016x}", h.finish()),
        }
    }
}

fn hex8(digest: &[u8]) -> String {
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

impl Write for Hasher {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.update(buf);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A 64-bit hash of a byte stream, defined here so that it never changes with a crate or
/// compiler version. The stream is cut into 16-byte blocks, the last one padded with zeros. Each
/// block `(x, y)` (two little-endian u64) updates the state `s` to `fold(x ^ P0, y ^ s)`, where
/// `fold` multiplies to 128 bits and xors the halves (as wyhash and foldhash do). The hash is
/// `fold(s ^ P2, length ^ P1)`.
///
/// Changing anything here changes every hash: record such a change as a new [`ViewHash`].
pub struct Fold64 {
    state: u64,
    len: u64,
    buf: [u8; BUF],
    /// Bytes waiting in `buf`. The stream before them is a whole number of blocks.
    n: usize,
}

const BLOCK: usize = 16;
const BUF: usize = 4 * BLOCK;
const P0: u64 = 0xa076_1d64_78bd_642f;
const P1: u64 = 0xe703_7ed1_a0b4_28db;
const P2: u64 = 0x8ebc_6af0_9c88_c6e3;

fn fold(a: u64, b: u64) -> u64 {
    let r = u128::from(a) * u128::from(b);
    (r as u64) ^ ((r >> 64) as u64)
}

impl Default for Fold64 {
    fn default() -> Self {
        Fold64::new()
    }
}

impl Fold64 {
    pub fn new() -> Fold64 {
        Fold64 { state: 0, len: 0, buf: [0; BUF], n: 0 }
    }

    fn block(state: u64, b: &[u8]) -> u64 {
        let x = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
        let y = u64::from_le_bytes(b[8..BLOCK].try_into().expect("8 bytes"));
        fold(x ^ P0, y ^ state)
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if self.n > 0 {
            let take = (BUF - self.n).min(data.len());
            self.buf[self.n..self.n + take].copy_from_slice(&data[..take]);
            self.n += take;
            data = &data[take..];
            if self.n < BUF {
                return;
            }
            for b in self.buf.chunks_exact(BLOCK) {
                self.state = Fold64::block(self.state, b);
            }
            self.n = 0;
        }
        let blocks = data.chunks_exact(BLOCK);
        let rest = blocks.remainder();
        for b in blocks {
            self.state = Fold64::block(self.state, b);
        }
        self.buf[..rest.len()].copy_from_slice(rest);
        self.n = rest.len();
    }

    pub fn finish(&self) -> u64 {
        let mut state = self.state;
        for b in self.buf[..self.n].chunks(BLOCK) {
            let mut last = [0u8; BLOCK];
            last[..b.len()].copy_from_slice(b);
            state = Fold64::block(state, &last);
        }
        fold(state ^ P2, self.len ^ P1)
    }
}

impl Write for Fold64 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// [`Fold64`] of the JSON of `v`.
fn json_hash<T: Serialize + ?Sized>(v: &T) -> u64 {
    let mut h = Fold64::new();
    serde_json::to_writer(&mut h, v).expect("views serialize");
    h.finish()
}

/// [`Fold64`] of each section's name, a zero byte and its hash as 8 little-endian bytes.
fn combine(sections: &[(&str, u64)]) -> u64 {
    let mut h = Fold64::new();
    for (name, x) in sections {
        h.update(name.as_bytes());
        h.update(&[0]);
        h.update(&x.to_le_bytes());
    }
    h.finish()
}

/// The [`ViewHash::Sections`] hash of views, and the hashes of the parts they share.
///
/// The hash of a view is [`Fold64`] of its fields in order, each as its JSON name, a zero byte
/// and the field's hash as 8 little-endian bytes. A field's hash is [`Fold64`] of its JSON,
/// except for `board`, whose hash combines its fields' hashes the same way, and `log` (see
/// [`Sections::log`]). So views with the same JSON have the same hash,
/// and two views whose JSON differs have the same hash with a chance of about 2^-64.
///
/// Views share the parts that are the same for every viewer, as `Arc`s
/// (`treachery_view::cache::ViewBatch`), and some across game states, like the faction
/// sheets. The hash of a shared part is computed once and remembered by the address of its
/// `Arc`. The memo holds a clone of the `Arc`, so that address can't be reused for another
/// value while the memo remembers it. [`Sections::end_batch`] forgets the parts the latest
/// views didn't use.
#[derive(Default)]
pub struct Sections {
    memo: HashMap<usize, Memo>,
    /// Counts the batches of views, to know which memo entries the latest one used.
    batch: u64,
}

struct Memo {
    _alive: Arc<dyn Any + Send + Sync>,
    hash: u64,
    batch: u64,
}

impl Sections {
    pub fn new() -> Sections {
        Sections::default()
    }

    /// The hash of a view, as 16 hex digits.
    pub fn hash(&mut self, view: &GameView) -> String {
        let sections = self.sections(view);
        format!("{:016x}", combine(&sections))
    }

    /// Forgets the shared parts that no view hashed since the last call. Call it after the views
    /// of a game state, so the memo holds only what the next views may share.
    pub fn end_batch(&mut self) {
        let batch = self.batch;
        self.memo.retain(|_, m| m.batch == batch);
        self.batch += 1;
    }

    /// The JSON name and hash of each field of `view`, in the order of its JSON. The test
    /// `sections_are_the_json_fields` checks the names.
    pub fn sections(&mut self, view: &GameView) -> Vec<(&'static str, u64)> {
        // Every field, so that a new one doesn't compile until it is hashed here.
        let GameView {
            event_number,
            version,
            turn,
            maximum_turns,
            maximum_players,
            main_phase,
            phase,
            viewer,
            players,
            me,
            board,
            status,
            actions,
            waiting_actions,
            log,
            ruleset,
            rules,
            rules_for_bots,
            economics_status,
            piles,
            tanks,
            bidding,
            battle,
            notes,
            timing,
            deals,
            bots_speed,
            tracker,
            mentat,
            ending,
            faction_sheets,
            rule_labels,
            recent_milestones,
        } = view;
        vec![
            ("eventNumber", json_hash(event_number)),
            ("version", json_hash(version)),
            ("turn", json_hash(turn)),
            ("maximumTurns", json_hash(maximum_turns)),
            ("maximumPlayers", json_hash(maximum_players)),
            ("mainPhase", json_hash(main_phase)),
            ("phase", json_hash(phase)),
            ("viewer", json_hash(viewer)),
            ("players", self.shared(players)),
            ("me", json_hash(me)),
            ("board", combine(&self.board(board))),
            ("status", json_hash(status)),
            ("actions", json_hash(actions)),
            ("waitingActions", json_hash(waiting_actions)),
            ("log", self.log(log)),
            ("ruleset", json_hash(ruleset)),
            ("rules", self.shared(rules)),
            ("rulesForBots", self.shared(rules_for_bots)),
            ("economicsStatus", json_hash(economics_status)),
            ("piles", json_hash(piles)),
            ("tanks", json_hash(tanks)),
            ("bidding", json_hash(bidding)),
            ("battle", json_hash(battle)),
            ("notes", json_hash(notes)),
            ("timing", self.shared(timing)),
            ("deals", json_hash(deals)),
            ("botsSpeed", json_hash(bots_speed)),
            ("tracker", json_hash(tracker)),
            ("mentat", self.shared(mentat)),
            (
                "ending",
                match ending {
                    Some(e) => self.shared(e),
                    None => json_hash(&()),
                },
            ),
            ("factionSheets", self.shared(faction_sheets)),
            ("ruleLabels", self.shared(rule_labels)),
            ("recentMilestones", json_hash(recent_milestones)),
        ]
    }

    /// The JSON name and hash of each field of the board, in the order of its JSON. Its hash
    /// combines them like the view's.
    pub fn board(&mut self, board: &BoardView) -> Vec<(&'static str, u64)> {
        let BoardView {
            sector_in_storm,
            shield_wall_destroyed,
            forces,
            spice,
            highlighted_territories,
            strongholds,
            terror,
            ambassadors,
            discoveries,
            attached,
            leaders,
            monsters,
            number_of_monsters,
            special_strongholds,
            atomics_aftermath,
            protected_from_storm,
            no_field_value,
            storm_prescience_moves,
            recent_moves,
        } = board;
        vec![
            ("sectorInStorm", json_hash(sector_in_storm)),
            ("shieldWallDestroyed", json_hash(shield_wall_destroyed)),
            ("forces", self.shared(forces)),
            ("spice", self.shared(spice)),
            ("highlightedTerritories", json_hash(highlighted_territories)),
            ("strongholds", self.shared(strongholds)),
            ("terror", json_hash(terror)),
            ("ambassadors", self.shared(ambassadors)),
            ("discoveries", json_hash(discoveries)),
            ("attached", self.shared(attached)),
            ("leaders", self.shared(leaders)),
            ("monsters", self.shared(monsters)),
            ("numberOfMonsters", json_hash(number_of_monsters)),
            ("specialStrongholds", self.shared(special_strongholds)),
            ("atomicsAftermath", json_hash(atomics_aftermath)),
            ("protectedFromStorm", self.shared(protected_from_storm)),
            ("noFieldValue", json_hash(no_field_value)),
            ("stormPrescienceMoves", json_hash(storm_prescience_moves)),
            ("recentMoves", self.shared(recent_moves)),
        ]
    }

    /// The log's hash: [`Fold64`] of `reports\0` and the hash of the headers, then `latest\0`,
    /// the number of reports and, for each, `index\0`, its index, `lines\0`, the number of lines
    /// and the hash of each line. Numbers are little-endian (the index 4 bytes, counts 8), and
    /// each hash is [`Fold64`] of the JSON, as 8 little-endian bytes. A line is shared by every
    /// viewer who may read it.
    fn log(&mut self, log: &LogView) -> u64 {
        let LogView { reports, latest } = log;
        let mut h = Fold64::new();
        h.update(b"reports\0");
        h.update(&self.shared(reports).to_le_bytes());
        h.update(b"latest\0");
        h.update(&(latest.len() as u64).to_le_bytes());
        for r in latest {
            let ReportLines { index, lines } = r;
            h.update(b"index\0");
            h.update(&index.to_le_bytes());
            h.update(b"lines\0");
            h.update(&(lines.len() as u64).to_le_bytes());
            for l in lines {
                h.update(&self.shared(l).to_le_bytes());
            }
        }
        h.finish()
    }

    /// [`Fold64`] of the JSON of a shared part, remembered.
    fn shared<T: Serialize + Send + Sync + 'static>(&mut self, part: &Arc<T>) -> u64 {
        let batch = self.batch;
        let m = self.memo.entry(Arc::as_ptr(part) as usize).or_insert_with(|| Memo { _alive: part.clone(), hash: json_hash(&**part), batch });
        m.batch = batch;
        m.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(n: usize) -> Vec<u8> {
        (0..n).map(|i| b"{\"view\":[1,2,3],\"x\":\"abc\"}"[i % 26]).collect()
    }

    /// The hash of a stream doesn't depend on how it is written.
    #[test]
    fn pieces_hash_like_the_whole() {
        for kind in [ViewHash::Sha256, ViewHash::Fold64] {
            for n in [0, 1, 15, 16, 17, 63, 64, 65, 200, 1000] {
                let data = text(n);
                let whole = kind.of(&data);
                for step in [1, 3, 7, 16, 50] {
                    let mut h = kind.hasher();
                    for piece in data.chunks(step) {
                        h.write_all(piece).unwrap();
                    }
                    assert_eq!(h.finish(), whole, "{kind:?}, {n} bytes in pieces of {step}");
                }
            }
        }
    }

    /// The hashes in recorded traces must never change.
    #[test]
    fn hashes_are_pinned() {
        assert_eq!(ViewHash::Sha256.of(b"abc"), "ba7816bf8f01cfea");
        assert_eq!(ViewHash::Fold64.of(b""), "e6d24d7e0390ae94");
        assert_eq!(ViewHash::Fold64.of(b"abc"), "39278e2fd636752b");
        assert_eq!(ViewHash::Fold64.of(&text(1000)), "11e51b5b9dfb9ade");
    }

    #[test]
    fn length_and_content_matter() {
        assert_ne!(ViewHash::Fold64.of(b"a"), ViewHash::Fold64.of(b"a\0"));
        assert_ne!(ViewHash::Fold64.of(&text(100)), ViewHash::Fold64.of(&text(101)[1..]));
    }
}
