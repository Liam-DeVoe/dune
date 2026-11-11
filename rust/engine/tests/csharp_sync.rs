//! Fails when the C# the port is derived from has changed since the port was
//! last synced with it (docs/csharp-sync.md).
//!
//! `tests/golden/csharp_sync.sha256` holds the SHA-256 of every `.cs` file in
//! `treachery-online/Shared` and `treachery-online/Bots`, as of the last sync.
//! The goldens, the generated modules and the coverage lists are all snapshots
//! of that C#, so a change there makes them stale without failing anything
//! else. This test names the files that changed. Follow the procedure in
//! docs/csharp-sync.md, then rewrite the manifest with
//!
//! ```sh
//! UPDATE_CSHARP_SYNC=1 cargo test -p treachery-engine --test csharp_sync
//! ```
//!
//! The second test checks that the oracle's fuzzer plays both sides of every
//! `Version <op> N` check in Shared.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const MANIFEST: &str = "tests/golden/csharp_sync.sha256";
const ROOTS: [&str; 2] = ["Shared", "Bots"];

fn csharp_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../treachery-online")
}

fn collect(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            if name != "bin" && name != "obj" {
                collect(&path, root, out);
            }
        } else if name.ends_with(".cs") {
            let bytes = std::fs::read(&path).unwrap();
            let hash: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.insert(rel, hash);
        }
    }
}

fn current() -> BTreeMap<String, String> {
    let root = csharp_root();
    let mut out = BTreeMap::new();
    for r in ROOTS {
        collect(&root.join(r), &root, &mut out);
    }
    out
}

fn render(files: &BTreeMap<String, String>) -> String {
    let mut s = String::from(
        "# SHA-256 of every C# file in Shared and Bots that the Rust port is synced to.\n\
         # Written by UPDATE_CSHARP_SYNC=1 cargo test -p treachery-engine --test csharp_sync\n\
         # after following docs/csharp-sync.md. Do not edit by hand.\n",
    );
    for (path, hash) in files {
        s.push_str(&format!("{hash}  {path}\n"));
    }
    s
}

fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let (hash, path) = l.split_once("  ").unwrap_or_else(|| panic!("bad manifest line: {l}"));
            (path.to_string(), hash.to_string())
        })
        .collect()
}

#[test]
fn csharp_sources_match_the_synced_manifest() {
    let now = current();
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(MANIFEST);
    if std::env::var_os("UPDATE_CSHARP_SYNC").is_some() {
        std::fs::write(&manifest_path, render(&now)).unwrap();
        return;
    }
    let synced = parse(&std::fs::read_to_string(&manifest_path).unwrap_or_default());

    let changed: Vec<_> = now.iter().filter(|(p, h)| synced.get(*p).is_some_and(|s| s != *h)).map(|(p, _)| p.as_str()).collect();
    let added: Vec<_> = now.keys().filter(|p| !synced.contains_key(*p)).map(String::as_str).collect();
    let removed: Vec<_> = synced.keys().filter(|p| !now.contains_key(*p)).map(String::as_str).collect();

    assert!(
        changed.is_empty() && added.is_empty() && removed.is_empty(),
        "The C# differs from what the Rust port is synced to. Port the change and regenerate every \
         artifact as described in rust/docs/csharp-sync.md, then update the manifest with\n  \
         UPDATE_CSHARP_SYNC=1 cargo test -p treachery-engine --test csharp_sync\n\
         changed: {changed:#?}\nadded: {added:#?}\nremoved: {removed:#?}"
    );
}

/// Every N in `Version <op> N` (the grep in Oracle/GameConfig.cs), for N >= 100.
fn version_gates_in_shared() -> BTreeMap<i32, String> {
    let mut files = BTreeMap::new();
    collect(&csharp_root().join("Shared"), &csharp_root(), &mut files);
    let mut gates = BTreeMap::new();
    for path in files.keys() {
        let text = std::fs::read_to_string(csharp_root().join(path)).unwrap();
        let bytes = text.as_bytes();
        let mut from = 0;
        while let Some(i) = text[from..].find("Version") {
            let mut j = from + i + "Version".len();
            from = j;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            let ops = ["<=", ">=", "==", "!=", "<", ">"];
            let Some(op) = ops.iter().find(|o| text[j..].starts_with(**o)) else { continue };
            j += op.len();
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            let start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                let n: i32 = text[start..j].parse().unwrap();
                if n >= 100 {
                    gates.entry(n).or_insert_with(|| path.clone());
                }
            }
        }
    }
    gates
}

#[test]
fn fuzzer_version_gates_cover_every_version_check() {
    let config = std::fs::read_to_string(csharp_root().join("Oracle/GameConfig.cs")).unwrap();
    let start = config.find("VersionGates =").expect("VersionGates in Oracle/GameConfig.cs");
    let body = &config[start..];
    let body = &body[body.find('[').unwrap() + 1..body.find(']').unwrap()];
    let listed: Vec<i32> = body.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.parse().unwrap()).collect();

    let missing: Vec<_> = version_gates_in_shared().into_iter().filter(|(n, _)| !listed.contains(n)).collect();
    assert!(missing.is_empty(), "version checks in Shared that Oracle/GameConfig.cs VersionGates does not list: {missing:?}");
}
