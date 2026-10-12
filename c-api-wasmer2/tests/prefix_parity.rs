//! The two backend C APIs must stay identical apart from their symbol prefix.
//!
//! This is what makes the duplication honest. The entry-point files have to be written out
//! literally — cbindgen parses the source syntactically, so a macro-generated `#[no_mangle]`
//! function produces no header entry — but nothing about them is allowed to *differ*. Their
//! bodies all forward into `c-api-common`, and the one real difference between the libraries
//! is each crate's `capi_backend`, which these files reach through a name that is the same on
//! both sides.
//!
//! A failure here means someone changed one copy and not the other. The fix is to re-derive
//! the file rather than to patch it twice.

/// `w7` and `w2` are the same length, so the substitution cannot change alignment or layout —
/// the comparison is a plain string equality, not a normalisation.
fn to_neutral_prefix(source: &str, prefix: &str) -> String {
    source.replace(prefix, "wN")
}

fn assert_parity(file_name: &str, wasmer2: &str, wasmer7: &str) {
    assert_eq!(
        to_neutral_prefix(wasmer2, "w2"),
        to_neutral_prefix(wasmer7, "w7"),
        "{file_name} has diverged between c-api-wasmer2 and c-api-wasmer7; \
         re-derive it instead of editing both",
    );
}

#[test]
fn executor_entry_points_match() {
    assert_parity(
        "capi_executor.rs",
        include_str!("../src/capi_executor.rs"),
        include_str!("../../c-api-wasmer7/src/capi_executor.rs"),
    );
}

#[test]
fn instance_entry_points_match() {
    assert_parity(
        "capi_instance.rs",
        include_str!("../src/capi_instance.rs"),
        include_str!("../../c-api-wasmer7/src/capi_instance.rs"),
    );
}

#[test]
fn instance_state_entry_points_match() {
    assert_parity(
        "capi_instance_state.rs",
        include_str!("../src/capi_instance_state.rs"),
        include_str!("../../c-api-wasmer7/src/capi_instance_state.rs"),
    );
}
