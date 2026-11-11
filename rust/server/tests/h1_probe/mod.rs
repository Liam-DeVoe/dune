//! Runs package H1's .NET probe (`server/dotnet-probe`) and holds the address generator shared
//! by `tests/mail_address.rs` and `tests/auth_dotnet.rs`.

#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use treachery_server::mail::MailAddress;

pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn dotnet() -> Command {
    let mut cmd = Command::new("dotnet");
    if let Some(home) = std::env::var_os("HOME") {
        let root = PathBuf::from(home).join("dotnet10");
        if root.exists() {
            let path = std::env::var_os("PATH").unwrap_or_default();
            let mut paths = vec![root.clone()];
            paths.extend(std::env::split_paths(&path));
            cmd.env("DOTNET_ROOT", &root).env("PATH", std::env::join_paths(paths).unwrap());
        }
    }
    cmd
}

/// Builds the probe and runs it in `mode` with `input` on stdin; returns stdout.
pub fn run_probe(mode: &str, input: &str) -> String {
    run_probe_with_env(mode, input, &[])
}

/// [`run_probe`] with extra environment variables (such as `TZ`).
pub fn run_probe_with_env(mode: &str, input: &str, env: &[(&str, &str)]) -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dotnet-probe");
    let status = dotnet().args(["build", "-c", "Release", "-nologo", "-v", "q"]).current_dir(&dir).status().expect("run dotnet build");
    assert!(status.success(), "dotnet build of the H1 probe failed");
    let mut child = dotnet()
        .arg(dir.join("bin/Release/net10.0/H1Probe.dll"))
        .arg(mode)
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("run the H1 probe");
    let mut stdin = child.stdin.take().unwrap();
    let input = input.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "H1 probe failed");
    String::from_utf8(output.stdout).unwrap()
}

/// `new MailAddress(address)` in the probe's output format.
pub fn rust_mail_result(address: &str) -> Value {
    match MailAddress::parse(address) {
        Ok(m) => json!({"ok": true, "user": m.user, "host": m.host, "display": m.display_name, "text": m.to_string()}),
        Err(e) => json!({"ok": false, "type": e.type_name, "msg": e.message}),
    }
}

/// xorshift64*: deterministic and dependency-free.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

const ATEXT: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&'*+-/=?^_`{|}~";
/// Characters with a role in the grammar.
const SPECIALS: &[&str] = &[".", "..", "@", "<", ">", "\"", "\\", "(", ")", "[", "]", ",", ";", ":", " ", "\t", "  "];
/// Non-ASCII and control characters: Latin, CJK, an astral char (a surrogate pair), combining
/// marks and compatibility characters that NFC changes, Unicode spaces, a BOM, NUL, DEL.
const UNICODE: &[&str] = &[
    "é", "ü", "ß", "Ä", "中", "文", "😀", "𝔸", "e\u{301}", "A\u{30a}", "\u{212b}", "\u{2126}", "\u{fb01}", "\u{a0}", "\u{2028}", "\u{3000}",
    "\u{feff}", "\u{200b}", "\u{0}", "\u{1}", "\u{7f}", "\u{1f}", "\u{b}", "\u{c}", "\u{85}", "\u{301}", "\u{130}", "\u{3a3}",
];
const LINE_BREAKS: &[&str] = &["\r\n", "\r\n ", "\n", "\r", "\r\n\t"];

fn atom(rng: &mut Rng) -> String {
    let max = if rng.chance(80) { 6 } else { 15 };
    let len = 1 + rng.below(max);
    let atext: Vec<char> = ATEXT.chars().collect();
    let mut s = String::new();
    for _ in 0..len {
        if rng.chance(6) {
            s.push_str(rng.pick(UNICODE));
        } else if rng.chance(70) {
            s.push(atext[rng.below(26)]);
        } else {
            s.push(*rng.pick(&atext));
        }
    }
    s
}

fn dot_atom(rng: &mut Rng) -> String {
    let n = 1 + rng.below(3);
    let mut parts: Vec<String> = (0..n).map(|_| atom(rng)).collect();
    if rng.chance(5) {
        parts.insert(rng.below(parts.len() + 1), String::new());
    }
    parts.join(".")
}

fn quoted_text(rng: &mut Rng) -> String {
    let mut s = String::new();
    for _ in 0..rng.below(8) {
        match rng.below(10) {
            0 => s.push_str("\\\""),
            1 => s.push_str("\\\\"),
            2 => s.push_str(rng.pick(UNICODE)),
            3 => s.push_str(rng.pick(SPECIALS)),
            4 => {
                s.push('\\');
                s.push_str(rng.pick(UNICODE));
            }
            _ => s.push_str(&atom(rng)),
        }
    }
    s
}

fn comment(rng: &mut Rng) -> String {
    let inner = match rng.below(6) {
        0 => comment(rng),
        1 => format!("{} {}", atom(rng), comment(rng)),
        2 => "\\)".to_string(),
        3 => format!("{}\\(", atom(rng)),
        _ => atom(rng),
    };
    format!("({inner})")
}

fn domain(rng: &mut Rng) -> String {
    match rng.below(10) {
        0 => format!("[{}]", quoted_text(rng).replace('"', "")),
        1 => format!("[{}.{}.{}.{}]", rng.below(300), rng.below(300), rng.below(300), rng.below(300)),
        _ => {
            let mut d = dot_atom(rng);
            if rng.chance(70) {
                d.push('.');
                d.push_str(rng.pick(&["com", "org", "de", "co.uk", "online", "中国"]));
            }
            d
        }
    }
}

fn local_part(rng: &mut Rng) -> String {
    if rng.chance(15) {
        format!("\"{}\"", quoted_text(rng))
    } else {
        dot_atom(rng)
    }
}

fn cfws(rng: &mut Rng) -> String {
    match rng.below(12) {
        0 => " ".to_string(),
        1 => "\t".to_string(),
        2 => comment(rng),
        3 => format!(" {} ", comment(rng)),
        4 => rng.pick(LINE_BREAKS).to_string(),
        5 => rng.pick(UNICODE).to_string(),
        _ => String::new(),
    }
}

fn display_name(rng: &mut Rng) -> String {
    match rng.below(4) {
        0 => format!("\"{}\"", quoted_text(rng)),
        1 => format!("{} {}", atom(rng), atom(rng)),
        2 => quoted_text(rng),
        _ => atom(rng),
    }
}

fn well_formed(rng: &mut Rng) -> String {
    let addr = format!("{}{}{}@{}{}{}", cfws(rng), local_part(rng), cfws(rng), cfws(rng), domain(rng), cfws(rng));
    match rng.below(6) {
        0 => format!("<{addr}>"),
        1 => format!("{} <{addr}>", display_name(rng)),
        2 => format!("{}<{addr}>{}", display_name(rng), cfws(rng)),
        3 => format!("{}{addr}", display_name(rng)),
        _ => addr,
    }
}

fn soup(rng: &mut Rng) -> String {
    let mut s = String::new();
    for _ in 0..1 + rng.below(12) {
        match rng.below(6) {
            0 | 1 => s.push_str(rng.pick(SPECIALS)),
            2 => s.push_str(rng.pick(UNICODE)),
            3 => s.push('@'),
            _ => s.push_str(&atom(rng)),
        }
    }
    s
}

/// Inserts, deletes, duplicates or swaps characters.
fn mutate(rng: &mut Rng, s: String) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    for _ in 0..1 + rng.below(3) {
        let i = rng.below(chars.len() + 1);
        match rng.below(5) {
            0 | 1 => {
                let piece = match rng.below(3) {
                    0 => rng.pick(SPECIALS).to_string(),
                    1 => rng.pick(UNICODE).to_string(),
                    _ => rng.pick(&["@", "\\", "\"", "(", ")", "<", ">", "[", "]", ".", ","]).to_string(),
                };
                chars.splice(i..i, piece.chars());
            }
            2 if i < chars.len() => {
                chars.remove(i);
            }
            3 if i < chars.len() => {
                let j = (i + 1 + rng.below(4)).min(chars.len());
                let dup: Vec<char> = chars[i..j].to_vec();
                chars.splice(j..j, dup);
            }
            4 if i + 1 < chars.len() => chars.swap(i, i + 1),
            _ => {}
        }
    }
    chars.into_iter().collect()
}

/// One address from the generator.
pub fn address(rng: &mut Rng) -> String {
    let s = match rng.below(20) {
        0..=2 => soup(rng),
        3..=5 => {
            let s = well_formed(rng);
            mutate(rng, s)
        }
        // Realistic, with the padding RequestCreateUser trims away (or not).
        6 => format!("{}.{}+{}@{}.example.com", atom(rng), atom(rng), atom(rng), atom(rng)).to_lowercase(),
        _ => well_formed(rng),
    };
    // Mostly avoid CR and LF: .NET rejects any address that has one, so they test one branch.
    if !s.is_empty() && (rng.chance(90) || !s.contains(['\r', '\n'])) {
        s.replace(['\r', '\n'], " ")
    } else {
        s
    }
}

/// Hand-picked cases, run before the generated ones.
pub const HAND_PICKED: &[&str] = &[
    "", " ", "a", "@", "a@b", "a@b.c", "user@example.com", " user@example.com ", "a@b..c", ".a@b", "a.@b", "a..b@c", "a@.b", "a@b.",
    "\"a b\"@c", "\"a\\\"b\"@c", "\"\"@c", "\"a@b", "a\"@b", "a@[1.2.3.4]", "a@[1.2.3.4", "a@1.2.3.4]", "a@[\\]]", "a@[\\é]", "a@[é]",
    "Name <a@b>", "\"Name\" <a@b>", "\"Na\\\"me\" <a@b>", "<a@b>", "a@b>", "<a@b", "<<a@b>>", "a@b (comment)", "(c)a@b", "a(c)@b",
    "a@(c)b", "a@b(c", "a@b c)", "a@b((c)", "a\\@b@c", "a@@b", "@b", "a@", "a@b,c@d", "a@b, c@d", "Name a@b", "Name, X <a@b>",
    "\"Name\" a@b", "\"Name\"a@b", "Name\"a@b", "é@ü.de", "用户@例子.广告", "😀@😀.com", "e\u{301}@x.com", "a@\u{212b}.com",
    "\u{212b}ngstr\u{f6}m <a@b>", "a@b\r\n", "a@b\n", "\r\na@b", "a b@c", "a@b c", "a@b\u{a0}", "\u{a0}a@b", "a@b\u{3000}",
    "\u{feff}a@b", "a\u{0}@b", "a@b\u{7f}", "a\\ b@c", "\"a\\\"@b", "Name <a@b> x", "x <a@b> <c@d>", "a@b.com;", ":a@b", "a@b:",
    "\\a@b", "\"\\\\\"@b", "a@-b.com", "a@b-.com", "-a@b", "a@b.c.d.e.f.g.h", "(((x)))a@b", "(\\()a@b", "(\\é)a@b", "a@b(\\é)",
    "Name <(c)a@b(c)>", "Name < a@b >", "Name <a @ b>", "\"a\" \"b\" <c@d>", "a.b.c@d.e.f", "admin@treachery.online",
];
