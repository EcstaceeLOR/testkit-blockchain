//! Ledger time control: advancing, warping, and evaluating contract state
//! at specific points in time, without reasoning about the
//! `timestamp`/`sequence` relationship by hand.
//!
//! # Example
//!
//! ```
//! use soroban_testkit::core::TestEnv;
//!
//! // Setup: a fresh environment and its starting timestamp.
//! let env = TestEnv::new();
//! let start = env.now();
//!
//! // Operation: advance the ledger clock by one hour.
//! env.advance_hours(1);
//!
//! // Assertion: the timestamp moved forward by exactly 3600 seconds.
//! assert_eq!(env.now(), start + 3600);
//! ```

mod clock;

pub use clock::LEDGER_CLOSE_TIME_SECS;
