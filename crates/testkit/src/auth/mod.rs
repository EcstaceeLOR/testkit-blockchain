//! Systematically prove that privileged entry points reject unauthorized
//! callers — the most common Soroban vulnerability class.
//!
//! # Example
//!
//! ```
//! use soroban_testkit::auth::AuthMatrix;
//! use soroban_testkit::core::TestEnv;
//!
//! // Setup: an environment and the one address allowed to call `withdraw`.
//! let env = TestEnv::new();
//! let owner = env.address();
//!
//! // Operation: a stand-in entry point that accepts only `owner`.
//! // Assertion: `assert_enforced` checks every allowed and known caller.
//! AuthMatrix::new(&env)
//!     .entry_point("withdraw", &[owner.clone()], move |caller| {
//!         if *caller == owner {
//!             Ok(())
//!         } else {
//!             Err(soroban_sdk::Error::from_contract_error(1))
//!         }
//!     })
//!     .assert_enforced();
//! ```

mod matrix;

pub use matrix::AuthMatrix;
