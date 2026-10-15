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
let buyer = env.address();
let seller = env.address();
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

`soroban-testkit audit` is a linter with three heuristics, not a security
product. It can miss real bugs and flag non-bugs; a missing finding is not a
security guarantee. This is the same bounded claim made by the command output
and the Module 9 requirements in [`BUILD_SPEC.md`](BUILD_SPEC.md#soroban-testkit-audit).
See [Audit command limitations](#audit-command).

`limits` ramps a numeric parameter directly, or generates addresses for a
`Vec<Address>` parameter — the common "how many recipients" question.
Every ramp attempt runs in its own subprocess: a real `.wasm` contract
that exceeds resource limits can abort the process outright rather than
return an error, and isolating each attempt is the only safe way to probe
past that boundary. Ledger read/write counts and transaction size aren't
reported (they come from a network-side simulated footprint this crate
doesn't produce); instructions and memory, measured locally, are. See
[Limits command limitations](#limits-command).

## Known limitations

These constraints follow from the current `soroban-sdk` test environment and
the project's network-free boundary. They are supported boundaries, not
claims about live-network behavior.

### TTL helpers

- TTL inspection and expiry use `soroban-sdk::testutils` in the current
  in-memory environment. They do not fetch a network's archival configuration
  or exercise restore-footprint operations.
- `ttl_of` follows the SDK behavior and panics when a persistent or temporary
  entry is missing or already expired. Instance storage has one TTL per
  contract, so its `key` argument is ignored.
- `expire(kind)` is intentionally coarse. It reads the environment's current
  `max_entry_ttl` and advances the whole ledger beyond it; `kind` documents the
  caller's intent but does not restrict expiry to one entry or storage kind.
  Use a disposable `TestEnv` when unrelated state must survive.

### Limits command

- `limits` registers the supplied WASM in a fresh local SDK environment for
  every probe, allows mocked authorization, and reports local CPU-instruction
  and memory budgets. It is a diagnostic ceiling, not a network simulation or
  a guarantee that the same call fits a live transaction.
- Ramp parameters are limited to integer types and `Vec<Address>`. Other
  parameters receive fixed defaults only for the supported primitive types;
  a parameter named `token` receives a funded SAC as a documented heuristic.
- Contracts must currently register with empty constructor arguments. Support
  for constructor arguments is tracked in [#242](https://github.com/soroban-testkit/testkit-blockchain/issues/242).
- Ledger read/write counts and transaction size are unavailable because they
  require the network-side simulated footprint that this command deliberately
  does not produce.

### Audit command

`audit` checks source syntax for three local patterns. It does not expand
macros, follow calls into helper functions, or replace a contract security
review, so both false positives and false negatives are expected.

## Roadmap

The supported surface today is deliberately in-process and network-free:
isolated test environments, ledger-clock control, adversarial money values,
event assertions, SAC token doubles, authorization matrices, TTL-expiry
checks, and the `coverage`, `limits`, and three-rule `audit` CLI commands.
The next increments are kept small and tracked in these curated issues:

| Area | Planned increment | Tracking issues |
|---|---|---|
| Ledger ergonomics | Sequence-oriented advancement and reusable clock checkpoints | [#60](https://github.com/soroban-testkit/testkit-blockchain/issues/60), [#61](https://github.com/soroban-testkit/testkit-blockchain/issues/61) |
| TTL assertions | Minimum-TTL and exact TTL-delta assertions | [#187](https://github.com/soroban-testkit/testkit-blockchain/issues/187), [#188](https://github.com/soroban-testkit/testkit-blockchain/issues/188) |
| Audit workflow | Rule selection, documented suppressions, baselines, and rule explanations | [#215](https://github.com/soroban-testkit/testkit-blockchain/issues/215), [#216](https://github.com/soroban-testkit/testkit-blockchain/issues/216), [#220](https://github.com/soroban-testkit/testkit-blockchain/issues/220), [#221](https://github.com/soroban-testkit/testkit-blockchain/issues/221) |
| Limits CLI | Constructor arguments for contracts that cannot be registered with `()` | [#242](https://github.com/soroban-testkit/testkit-blockchain/issues/242) |
| Real-contract validation | Recurring-contract and batch-payout authorization coverage | [#266](https://github.com/soroban-testkit/testkit-blockchain/issues/266), [#267](https://github.com/soroban-testkit/testkit-blockchain/issues/267) |

Issues describe intended work, not release promises. Mainnet/testnet forking,
deployment tooling, frontend testing, a test runner, a full benchmarking
framework, and network-dependent test behavior remain out of scope; see
[`BUILD_SPEC.md` §3](BUILD_SPEC.md#3-scope-boundaries).

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
