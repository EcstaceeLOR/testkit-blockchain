//! Public API inventory (Issue #279).
//!
//! `prelude.rs` is the documented, everyday entry point
//! (`use soroban_testkit::prelude::*;`), but nothing previously caught an
//! accidental removal of one of its re-exports: `cargo build` on the crate
//! itself still succeeds even if a `pub use` line disappears, since it's
//! the *consumer's* `use` that would fail to resolve.
//!
//! This test imports every symbol through `prelude::*` exactly as a
//! real test file would, and constructs or references each one, so that
//! removing (or failing to add) a primary type here is a compile error in
//! this crate's own test suite, not something only surfaces downstream.
//!
//! If a new primary, documented type is added to the crate, add it here too.

use soroban_testkit::prelude::*;

#[test]
fn prelude_exports_every_documented_primary_type() {
    // core::{TestEnv, TestkitError}
    let env = TestEnv::new();
    let _: Option<TestkitError> = None;

    // auth::AuthMatrix
    let _matrix = AuthMatrix::new(&env);

    // events::{CapturedEvent, EventLog}
    let _log: EventLog = env.events();
    let _: Option<CapturedEvent> = None;

    // money::{adversarial_amounts, amounts_in, bps_values, Conservation}
    let _: Vec<i128> = adversarial_amounts();
    let _: Vec<i128> = amounts_in(&env, 0, 100, 1);
    let _: Vec<u32> = bps_values();
    let _: Option<Conservation> = None;

    // tokens::TestToken
    let _token: TestToken = env.token();

    // ttl::StorageKind
    let _kind = StorageKind::Persistent;
}
