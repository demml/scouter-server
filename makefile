PROJECT=scouter-server
SOURCE_OBJECTS=src

cargo.format:
	cargo fmt
cargo.lints:
	cargo clippy --workspace --all-targets -- -D warnings
cargo.test:
	cargo test

cargo.bench:
	cargo bench

test.unit:
	poetry run pytest \
		--cov \
		--cov-fail-under=0 \
		--cov-report xml:./coverage.xml \
		--cov-report term 
