//! Panic capture. Every panic during a replay is recorded with its message and
//! source location, so a `todo!` deep in the engine can be reported as
//! "not ported yet: <message> at <file:line>" and grouped across traces.

use std::backtrace::Backtrace;
use std::cell::{Cell, RefCell};
use std::panic;
use std::sync::Once;

use treachery_engine::compat::cs::CsException;

/// The last panic on this thread.
#[derive(Clone, Debug, Default)]
pub struct PanicInfo {
    pub message: String,
    pub location: String,
    /// Set when the payload was a C# exception.
    pub cs: Option<CsException>,
    /// Captured only while [`capture_backtraces`] is on.
    pub backtrace: Option<String>,
}

thread_local! {
    static LAST: RefCell<Option<PanicInfo>> = const { RefCell::new(None) };
    static BACKTRACE: Cell<bool> = const { Cell::new(false) };
}

/// Installs the hook (once per process). Panics are recorded and not printed,
/// unless `DIFFER_PRINT_PANICS` is set.
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let print = std::env::var_os("DIFFER_PRINT_PANICS").is_some();
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let payload = info.payload();
            let cs = crate::replay::as_exception(payload);
            let message = if let Some(e) = &cs {
                format!("{}: {}", treachery_engine::compat::cs::full_type_name(e.type_name), e.message)
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "<non-string panic payload>".to_string()
            };
            let location = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
            let backtrace = if cs.is_some() && BACKTRACE.with(|b| b.get()) {
                Some(Backtrace::force_capture().to_string())
            } else {
                None
            };
            LAST.with(|l| *l.borrow_mut() = Some(PanicInfo { message, location, cs, backtrace }));
            if print {
                previous(info);
            }
        }));
    });
}

/// Takes the last recorded panic on this thread.
pub fn take_last() -> Option<PanicInfo> {
    LAST.with(|l| l.borrow_mut().take())
}

/// While on, C# exceptions capture a backtrace (slow; used to classify the
/// exceptions of events that override `Execute`).
pub fn capture_backtraces(on: bool) {
    BACKTRACE.with(|b| b.set(on));
}
