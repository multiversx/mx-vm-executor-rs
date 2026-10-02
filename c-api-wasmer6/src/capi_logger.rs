//! Panic suppression at the C boundary.
//!
//! The module is named `capi_logger` because that is the path the shared
//! `meta::capi_safe_unwind` attribute expands to. It deliberately does not carry the legacy
//! crate's logging FFI: log configuration belongs to the process, and the legacy `c-api`
//! already owns those symbols.

use std::sync::Once;

static PANIC_HANDLER: Once = Once::new();

/// Installs a no-op panic hook, so a caught panic does not also print a backtrace.
///
/// A panic must never unwind across `extern "C"`; the `capi_safe_unwind` attribute catches
/// it and turns it into `VM_EXEC_W6_PANIC`. This only silences the default hook's output.
pub fn set_panic_handler() {
    PANIC_HANDLER.call_once(|| {
        std::panic::set_hook(Box::new(|_| {}));
    });
}
