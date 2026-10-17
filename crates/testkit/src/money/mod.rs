//! Adversarial value generation and conservation checking for money math —
//! the arithmetic bugs hand-written tests tend to miss.
//!
//! # Example
//!
//! ```
//! use soroban_testkit::money::{adversarial_amounts, Conservation};
//!
//! // Setup: edge-case amounts, including zero, for the code under test.
//! let amounts = adversarial_amounts();
//! assert!(amounts.contains(&0));
//!
//! // Operation: a deposit split into a withdrawal, a refund and a remainder.
//! let deposited = 1_000_000_000_i128;
//! let withdrawn = 600_000_000_i128;
//! let refunded = 100_000_000_i128;
//! let remaining = deposited - withdrawn - refunded;
//!
//! // Assertion: no value was created or destroyed.
//! Conservation { deposited, withdrawn, refunded, remaining }.assert_holds();
//! ```

mod generators;
mod invariants;

pub use generators::{adversarial_amounts, amounts_in, bps_values, overflow_edge_triples};
pub use invariants::{assert_conserved_over, Conservation};
