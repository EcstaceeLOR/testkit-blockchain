# Architecture

A map of the workspace for contributors: what each crate and module is
responsible for, how they depend on each other, and the conventions that
hold the codebase together. Read `BUILD_SPEC.md` first for the project's
scope and non-negotiable rules; this document is about *where things live*
and *why the boundaries are drawn where they are*.

## Workspace layout

```
soroban-testkit/
├── crates/
│   ├── testkit/     the library: soroban-testkit
│   └── cli/         the binary: soroban-testkit-cli (coverage, limits, audit)
├── examples/
│   └── vault/       a real contract used as a fixture by the CLI's
│                     integration tests (compiled to wasm32v1-none)
└── deny.toml, .github/workflows/ci.yml, BUILD_SPEC.md, CONTRIBUTING.md
```

Two workspace members, `crates/*` and `examples/vault`, resolved with
`resolver = "2"` (see the root `Cargo.toml`). `examples/vault` is not a
toy — the CLI's `limits` command builds it to a real `.wasm` and asserts
against it, so it doubles as a fixture and as living documentation of what
a testkit-tested contract looks like.

## `crates/testkit` — the library

Everything here is a `pub mod`, declared in `lib.rs`, each owning one
concern. A module never reaches into another module's private internals;
cross-module composition happens through `prelude.rs` or through public
types passed as arguments (e.g. `TestEnv` is threaded through nearly every
other module's public functions).

| Module | Owns | Depends on |
|---|---|---|
| `core` | [`TestEnv`] (wraps `soroban_sdk::Env`) and [`TestkitError`], the error type every assertion helper panics with | — (the foundation) |
| `auth` | [`AuthMatrix`] — systematically proving every privileged entry point rejects unauthorized callers | `core` |
| `events` | Asserting on emitted events without hand-decoding `Val` topics | `core` |
| `ledger` | Advancing/warping ledger time and reasoning about the `timestamp`/`sequence` relationship | `core` |
| `money` | Adversarial `i128` generation and conservation-of-value assertions | `core` |
| `tokens` | One-line Stellar Asset Contract test doubles | `core`, `ledger` |
| `ttl` | Ledger-entry TTL inspection and expiry simulation | `core`, `ledger` |
| `prelude` | Re-exports the common subset of the above for a single `use soroban_testkit::prelude::*;` | all of the above |

If you're adding a new capability, ask which row it belongs in before
writing code. A new top-level module is warranted only when the capability
doesn't fit an existing one *and* is itself in scope per `BUILD_SPEC.md`
§3 — most new assertion helpers extend an existing module (e.g. a new
event-shape assertion goes in `events`, not a new module).

### The error contract

Assertion helpers **panic** — that's how a failing test surfaces in
`cargo test` output — but every panic carries a [`TestkitError`], never a
bare string. This is what makes failure messages a first-class feature
(per `CONTRIBUTING.md`): a `TestkitError::Misuse("events were never
captured".into())` tells the next contributor exactly what to fix, where a
bare `panic!("bad state")` doesn't. When you add an assertion helper, add
its failure variant to `TestkitError` (or reuse an existing one) rather
than reaching for `panic!` directly.

### Public error codes

`TestkitError::code()` is the stable, machine-readable identifier for a
failure. The message remains intended for people and may change; codes are
part of the compatibility contract and must never be reused for a different
variant. Keep this table synchronized with the `match` in
[`crates/testkit/src/core/error.rs`](crates/testkit/src/core/error.rs) and
extend the unit test there when adding a variant.

| Variant | Code | Meaning |
|---|---|---|
| `TestkitError::AssertionFailed` | `TESTKIT_ASSERTION_FAILED` | An assertion about contract state or behavior was not met. |
| `TestkitError::DecodeFailed` | `TESTKIT_DECODE_FAILED` | A captured value could not be decoded into the requested type. |
| `TestkitError::Misuse` | `TESTKIT_MISUSE` | A testkit API was used outside its documented contract. |

The codes identify the variant only; the human-readable detail remains in
the error's display message. New variants get new codes, and existing codes
do not change.

## `crates/cli` — the binary

A thin layer over the library, structured as one file per subcommand under
`src/commands/`:

| Command | Purpose |
|---|---|
| `coverage` | Runs and reports test coverage for a contract crate |
| `limits` | Builds a contract to `wasm32v1-none` and checks it against Soroban resource limits (this is what depends on `examples/vault`) |
| `audit` | Runs the auth/event/TTL assertion suite against a contract as a single pass |

`main.rs` wires subcommands to `clap`'s derive API and nothing else — CLI
argument parsing and output formatting live in `commands/`, and any logic
that isn't specific to being a CLI (e.g. "how do I build a contract to
wasm") belongs in the `testkit` library so it stays usable outside the
CLI.

## Dependency boundaries

The library and the CLI are independent packages: neither depends on the
other, and both only depend on `soroban-sdk` for contract-facing code.
Keep it that way — a dependency that crosses this boundary is a design
change, not a routine `cargo add`.

| Package | Normal dependencies | Dev-dependencies |
|---|---|---|
| `soroban-testkit` (`crates/testkit`) | `soroban-sdk` (feature `testutils`), `thiserror` | `vault` (`examples/vault`, feature `testutils`) |
| `soroban-testkit-cli` (`crates/cli`) | `soroban-sdk` (feature `testutils`), `soroban-spec`, `clap`, `syn`, `quote`, `proc-macro2`, `walkdir` | — |

What this means in practice:

- **`soroban-testkit` has no Cargo features of its own.** `soroban-sdk`'s
  `testutils` feature is always enabled, so the library is only meaningful
  as a `[dev-dependencies]` entry in a contract crate. Do not add it under
  `[dependencies]` of a contract.
- **CLI-only crates stay in `crates/cli`.** `syn`, `walkdir`, `clap`, and
  `soroban-spec` must never be pulled into `crates/testkit`; the library
  must remain usable without a parser or argument-handling stack.
- **The CLI never links the library.** Logic shared by both belongs in
  `soroban-testkit`, and the CLI calls it through public items only.
- **`examples/vault` is a dev-only fixture.** It appears in the library's
  dev-dependencies and is built by the CLI's integration tests as a
  subprocess (`cargo build -p vault`), not as a Cargo dependency of the CLI.

To verify the map locally:

```sh
cargo tree -p soroban-testkit -e normal          # no soroban-testkit-cli
cargo tree -p soroban-testkit-cli -e normal      # no soroban-testkit
cargo tree -p soroban-testkit -e dev             # vault appears only here
```

CI's `lint` job runs the first two checks and fails if either package
depends on the other.

## Data flow: a typical test

1. A contract test constructs a [`TestEnv`] (`core`), which wraps
   `soroban_sdk::Env` and is the handle every other module's functions
   take as their first argument.
2. The test drives the contract under test as normal (`soroban_sdk`
   client calls), then reaches for `soroban_testkit::prelude::*` helpers
   to assert on the result: `assert_conserved_over` (`money`), an events
   assertion (`events`), a TTL check (`ttl`), or `AuthMatrix` (`auth`) to
   prove unauthorized callers are rejected.
3. On failure, the helper panics with a `TestkitError`-derived message
   that names the specific expectation and the actual value observed —
   never a generic "assertion failed".

## CI's shape mirrors this structure

`.github/workflows/ci.yml` jobs map directly onto the guarantees this
document describes: `lint`/`test`/`doc` enforce the code conventions in
`CONTRIBUTING.md`, `audit`/`deny` enforce the supply-chain policy, and
`coverage`/`scheduled-sdk-check` guard the two properties above the module
table can't: how much of the library's own behavior is exercised by its
own test suite, and whether it still builds against whatever `soroban-sdk`
release is current upstream.

[`TestEnv`]: crates/testkit/src/core/env.rs
[`TestkitError`]: crates/testkit/src/core/error.rs
[`AuthMatrix`]: crates/testkit/src/auth/matrix.rs
