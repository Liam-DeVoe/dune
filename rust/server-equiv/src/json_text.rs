//! The transcript line builder, the twin of `ServerOracle/JsonText.cs`. Strings are escaped as
//! System.Text.Json's default encoder does, and serialized values go in verbatim, so a Rust
//! transcript line is byte-identical to the C# one when the hubs agree.

use sha2::{Digest, Sha256};
use treachery_engine::json::{serialize, write_escaped, ToJson};

#[derive(Default)]
pub struct JsonText {
    out: String,
    first: Vec<bool>,
    pending_value: bool,
}

impl JsonText {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish(self) -> String {
        self.out
    }

    fn before_value(&mut self) {
        if self.pending_value {
            self.pending_value = false;
            return;
        }
        if let Some(first) = self.first.last_mut() {
            if !*first {
                self.out.push(',');
            }
            *first = false;
        }
    }

    pub fn obj_start(&mut self) -> &mut Self {
        self.before_value();
        self.out.push('{');
        self.first.push(true);
        self
    }

    pub fn obj_end(&mut self) -> &mut Self {
        self.first.pop();
        self.out.push('}');
        self
    }

    pub fn arr_start(&mut self) -> &mut Self {
        self.before_value();
        self.out.push('[');
        self.first.push(true);
        self
    }

    pub fn arr_end(&mut self) -> &mut Self {
        self.first.pop();
        self.out.push(']');
        self
    }

    pub fn key(&mut self, key: &str) -> &mut Self {
        self.before_value();
        write_escaped(&mut self.out, key);
        self.out.push(':');
        self.pending_value = true;
        self
    }

    /// Inserts already serialized JSON.
    pub fn raw(&mut self, json: &str) -> &mut Self {
        self.before_value();
        self.out.push_str(json);
        self
    }

    pub fn str(&mut self, s: Option<&str>) -> &mut Self {
        let e = escape(s);
        self.raw(&e)
    }

    pub fn int(&mut self, i: i64) -> &mut Self {
        self.raw(&i.to_string())
    }

    pub fn null(&mut self) -> &mut Self {
        self.raw("null")
    }

    /// Any value as `Utilities.Serialize` writes it.
    pub fn ser<T: ToJson + ?Sized>(&mut self, value: &T) -> &mut Self {
        let s = serialize(value);
        self.raw(&s)
    }

    pub fn member(&mut self, key: &str, s: Option<&str>) -> &mut Self {
        self.key(key).str(s)
    }

    pub fn member_raw(&mut self, key: &str, json: &str) -> &mut Self {
        self.key(key).raw(json)
    }

    pub fn member_ser<T: ToJson + ?Sized>(&mut self, key: &str, value: &T) -> &mut Self {
        self.key(key).ser(value)
    }
}

/// `JsonSerializer.Serialize(string?)`.
pub fn escape(s: Option<&str>) -> String {
    match s {
        None => "null".to_string(),
        Some(s) => {
            let mut out = String::new();
            write_escaped(&mut out, s);
            out
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
