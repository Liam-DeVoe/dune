//! Allowed differences: rules that make the replayer accept a deliberate change while it is
//! being reviewed (native-harness.md, "Deliberate behavior changes"). One rule per line:
//!
//! ```text
//! # comment
//! log <pattern>       log messages matching the pattern are left out on both sides
//! message <pattern>   rejection messages matching it (on either side) compare equal
//! view <path>         this view path and everything below it is not compared
//! bots                bot decisions are not compared
//! ```
//!
//! Patterns match the whole text, with `*` for any run of characters. View paths are JSON
//! paths such as `/status/message` or `/actions/*/validation` (`*` matches one path segment).

#[derive(Default, Debug)]
pub struct Allow {
    log: Vec<String>,
    message: Vec<String>,
    view: Vec<Vec<String>>,
    pub bots: bool,
}

/// Whole-text match with `*` wildcards.
pub fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let mut rest = text;
    for (i, p) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(p) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(p);
        } else {
            match rest.find(p) {
                Some(at) => rest = &rest[at + p.len()..],
                None => return false,
            }
        }
    }
    true
}

impl Allow {
    pub fn parse(text: &str) -> Result<Allow, String> {
        let mut a = Allow::default();
        for (n, raw) in text.lines().enumerate() {
            let l = raw.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let (kind, arg) = l.split_once(char::is_whitespace).map(|(k, r)| (k, r.trim())).unwrap_or((l, ""));
            match kind {
                "log" => a.log.push(arg.to_string()),
                "message" => a.message.push(arg.to_string()),
                "view" => a.view.push(arg.trim_matches('/').split('/').map(str::to_string).collect()),
                "bots" => a.bots = true,
                _ => return Err(format!("allow file line {}: unknown rule {kind}", n + 1)),
            }
        }
        Ok(a)
    }

    pub fn load(path: Option<&str>) -> Result<Allow, String> {
        match path {
            None => Ok(Allow::default()),
            Some(p) => Allow::parse(&std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?),
        }
    }

    pub fn ignores_log(&self, text: &str) -> bool {
        self.log.iter().any(|p| glob(p, text))
    }

    pub fn same_message(&self, expected: Option<&str>, actual: Option<&str>) -> bool {
        expected == actual || [expected, actual].iter().flatten().any(|m| self.message.iter().any(|p| glob(p, m)))
    }

    pub fn has_view_rules(&self) -> bool {
        !self.view.is_empty()
    }

    pub fn ignores_view(&self, path: &str) -> bool {
        let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
        self.view.iter().any(|rule| rule.len() <= segs.len() && rule.iter().zip(&segs).all(|(r, s)| r == "*" || r == s))
    }
}
