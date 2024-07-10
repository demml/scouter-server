PROJECT=scouter-server
SOURCE_OBJECTS=src

cargo.format:
	cargo fmt
cargo.lints:
	cargo clippy --workspace --all-targets -- -D warnings
cargo.test:
	cargo test -- --nocapture  --test-threads=1

cargo.bench:
	cargo bench

