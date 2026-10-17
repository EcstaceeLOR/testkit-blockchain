//! One-line Stellar Asset Contract test doubles.
//!
//! # Example
//!
//! ```
//! use soroban_testkit::core::TestEnv;
//!
//! // Setup: a fresh environment, a standard token and a recipient.
//! let env = TestEnv::new();
//! let token = env.token();
//! let alice = env.address();
//!
//! // Operation: mint 500 units to `alice`.
//! token.mint(&alice, 500);
//!
//! // Assertion: her balance reflects the mint.
//! token.assert_balance(&alice, 500);
//! ```

mod sac;

pub use sac::TestToken;
