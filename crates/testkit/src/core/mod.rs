//! The entry point every other module builds on: [`TestEnv`], a wrapper
//! around [`soroban_sdk::Env`], and [`TestkitError`], the error type
//! assertion helpers panic with.
//!
//! # Example
//!
//! ```
//! use soroban_testkit::core::TestEnv;
//!
//! // Setup: a fresh environment.
//! let env = TestEnv::new();
//!
//! // Operation: derive two addresses from it.
//! let alice = env.address();
//! let bob = env.address();
//!
//! // Assertion: each call yields a distinct address.
//! assert_ne!(alice, bob);
//! ```

mod env;
mod error;

pub use env::TestEnv;
pub use error::TestkitError;
