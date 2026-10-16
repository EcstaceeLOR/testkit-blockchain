#![doc = include_str!("../../../README.md")]
#![deny(missing_docs)]

// This crate root intentionally re-exports nothing. Primary types
// (`TestEnv`, `TestkitError`, `EventLog`, `Conservation`, `AuthMatrix`,
// `TestToken`, `StorageKind`, …) are not available as `soroban_testkit::Foo`.
// Reach them either through `soroban_testkit::prelude::*` (the supported
// everyday path) or through their owning module, e.g.
// `soroban_testkit::core::TestEnv`, for callers who want explicit paths
// instead of a glob import. See `prelude` for the full re-export list.
pub mod auth;
pub mod core;
pub mod events;
pub mod ledger;
pub mod money;
pub mod prelude;
pub mod tokens;
pub mod ttl;
