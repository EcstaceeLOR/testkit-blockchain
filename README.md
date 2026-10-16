# soroban-testkit

**The testing crate you would have written yourself, on the third contract.**

Testing infrastructure for Soroban contracts: property-test generators for
money math, an event-assertion API, ledger time and TTL-expiry control,
token test doubles, and a coverage-aware CLI.

`soroban-sdk` ships `testutils`, which is adequate for basic unit tests. It
is not adequate for testing contracts that hold money. Every serious
Soroban contract repo independently rebuilds some subset of:

- Advancing ledger time correctly, including the interaction between
  `timestamp` and `sequence`
- Asserting on emitted events without hand-decoding `Val` topics
- Generating adversarial `i128` values that actually find overflow bugs
- Deploying and funding a Stellar Asset Contract token double for transfer
  tests
- Proving that every privileged entry point rejects unauthorized callers
- Simulating TTL expiry

`soroban-testkit` is that shared layer, as a dev-dependency crate plus a
small CLI.

## Example

A small escrow contract, tested end to end: mint a token, release it to
the seller, and check the event, the balances, and that no value was
created or destroyed along the way.

```
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};
use soroban_testkit::prelude::*;

#[contract]
struct Escrow;

#[contractimpl]
impl Escrow {
    pub fn release(env: Env, token: Address, from: Address, to: Address, amount: i128) {
        from.require_auth();
        soroban_sdk::token::TokenClient::new(&env, &token)
            .transfer(&from, soroban_sdk::MuxedAddress::from(to), &amount);
        #[allow(deprecated)]
        env.events().publish((symbol_short!("release"),), amount);
    }
}

# fn main() {
let env = TestEnv::new();
let token = env.token();
let [buyer, seller]: [soroban_sdk::Address; 2] =
    env.addresses(2).try_into().unwrap();
token.mint(&buyer, 100);

let escrow_id = env.env().register(Escrow, ());
let client = EscrowClient::new(env.env(), &escrow_id);

env.env().mock_all_auths();
let (_, events) = env.events_during(|| {
    client.release(&token.address(), &buyer, &seller, &1_000_000_000);
});
events
    .from(&escrow_id)
    .assert_emitted(symbol_short!("release"));

// Each of these balance queries is itself a top-level call, so check
// them after reading events — see EventLog's note on scope.
token.assert_balance(&buyer, 0);
token.assert_balance(&seller, 1_000_000_000);

Conservation {
    deposited: 1_000_000_000,
    withdrawn: 1_000_000_000,
    refunded: 0,
    remaining: 0,
}
.assert_holds();
# }
```

## CLI

A companion binary, `soroban-testkit-cli` (installs as `soroban-testkit`),
for things that don't belong in a dev-dependency:

```sh
# Coverage for the WASM target (wraps cargo-llvm-cov, which needs no
# Soroban-specific flags since contract tests run natively).
soroban-testkit coverage --format html --open

# Empirically find how many recipients a batch operation can handle
# before it exceeds mainnet resource limits.
soroban-testkit limits --contract target/wasm32v1-none/release/my_contract.wasm \
  --fn batch_payout --ramp recipients

# Static checks: missing require_auth, unchecked i128 arithmetic,
# storage reads with no TTL bump. Not a security product.
soroban-testkit audit ./src --strict
```

`limits` ramps a numeric parameter directly, or generates addresses for a
`Vec<Address>` parameter — the common "how many recipients" question.
Every ramp attempt runs in its own subprocess: a real `.wasm` contract
that exceeds resource limits can abort the process outright rather than
return an error, and isolating each attempt is the only safe way to probe
past that boundary. Ledger read/write counts and transaction size aren't
reported (they come from a network-side simulated footprint this crate
doesn't produce); instructions and memory, measured locally, are.

`audit` reports each finding's path relative to the audited `PATH`, with
`/` separators, so `soroban-testkit audit src` run from the crate and
`soroban-testkit audit my_contract/src` run from its parent both print
`lib.rs:12: ...`. When `PATH` is a single file, its file name is used.

### Coverage thresholds

`--fail-under PCT` makes `coverage` exit non-zero when **line** coverage is
below `PCT`. It is forwarded unchanged to cargo-llvm-cov as
`--fail-under-lines PCT`, so cargo-llvm-cov does the measuring and the
comparison; it combines with any `--format`. Requires
`cargo install cargo-llvm-cov`. Running the command locally makes no
network access.

```sh
# Exits 0 if line coverage is >= 80%, otherwise prints the report and
# fails with: error: coverage run failed (cargo llvm-cov exited with ...)
soroban-testkit coverage --fail-under 80

# Same threshold, writing lcov.info for an IDE or CI upload step.
soroban-testkit coverage --format lcov --fail-under 92.5

# What actually runs (shown with --verbose):
#   cargo llvm-cov test --fail-under-lines 80
soroban-testkit --verbose coverage --fail-under 80
```

In CI, run it as an ordinary step; its exit code gates the job.

### Output controls

These global flags go before or after the subcommand:

| Flag | Effect |
| --- | --- |
| `--no-color` | No ANSI color codes, including from the `cargo` that `coverage` runs. A non-empty `NO_COLOR` environment variable does the same. Color is only ever written to a terminal, so piped output is already plain. |
| `-q`, `--quiet` | Results and errors only: drops advisory notes such as the `audit` disclaimer and "no findings". Passes `--quiet` to cargo-llvm-cov. |
| `-v`, `--verbose` | Extra diagnostics on stderr (files scanned, commands run, each `limits` probe). Passes `--verbose` to cargo-llvm-cov. Stdout is unchanged. |

`--quiet` and `--verbose` cannot be combined. Findings, measured limits,
`--json` documents, and errors are printed at every level.

## Offline install

Contributors working without network access to crates.io can prefetch and
verify dependencies ahead of time, then build and test fully offline:

```sh
# On a machine with network access: fetch every dependency into the
# local Cargo registry cache and vendor it alongside the workspace.
cargo vendor --locked vendor > cargo-vendor-config.toml

# Copy `vendor/` and `cargo-vendor-config.toml` to the offline machine,
# then point Cargo at the vendored sources instead of crates.io by
# appending the generated config to .cargo/config.toml:
mkdir -p .cargo
cat cargo-vendor-config.toml >> .cargo/config.toml
```

With the vendor directory in place, `cargo build`, `cargo test
--workspace`, and `cargo doc --workspace` all resolve dependencies from
`vendor/` with no network access. `cargo vendor --locked` fails loudly if
`Cargo.lock` and the registry disagree, so re-run it after any dependency
bump rather than editing `vendor/` by hand. Verify the result before
relying on it:

```sh
cargo test --workspace --offline
```

`--offline` makes Cargo error immediately if any dependency would need a
network fetch, instead of silently falling back to it.

## Supported versions

- **Rust:** the `stable` channel pinned in
  [`rust-toolchain.toml`](rust-toolchain.toml). There is no separate MSRV —
  the pinned stable toolchain is the only version CI runs against. See
  [CONTRIBUTING.md's Rust edition policy](CONTRIBUTING.md#rust-edition-policy)
  for how and when that pin moves.
- **`soroban-sdk`:** the version pinned in `[workspace.dependencies]` in the
  root `Cargo.toml` is the only version this crate is tested against for
  every PR. A weekly `scheduled-sdk-check` CI job additionally builds and
  tests the workspace against whatever version crates.io currently
  considers latest, independent of that pin, and opens a tracking issue if
  a newer release breaks compatibility — so drift is caught before a
  contributor hits it, without requiring every PR to track upstream.

## Status

This crate is under active development. See `BUILD_SPEC.md` for the build
plan and module boundaries, and [`ARCHITECTURE.md`](ARCHITECTURE.md) for a
contributor-facing map of the workspace and how its modules fit together.

## Prior art

[`soroban-fork`](https://crates.io/crates/soroban-fork) does lazy
mainnet/testnet forking for tests. That is a different problem;
`soroban-testkit` does not attempt it.

## License

Apache-2.0
