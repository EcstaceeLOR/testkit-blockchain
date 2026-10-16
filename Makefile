.PHONY: check fmt lint test doc docsrs audit deny cli build clean

check: fmt lint test doc docsrs audit deny cli

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Mirrors [package.metadata.docs.rs] in crates/testkit/Cargo.toml.
docsrs:
	RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc -p soroban-testkit --no-deps --target x86_64-unknown-linux-gnu

audit:
	cargo audit

deny:
	cargo deny check

cli:
	cargo test -p soroban-testkit-cli

clean:
	cargo clean
