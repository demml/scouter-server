PROJECT=scouter-server
SOURCE_OBJECTS=src

format:
	cargo fmt
lints:
	cargo clippy --workspace --all-targets -- -D warnings
test:
	cargo test -- --nocapture  --test-threads=1

setup-local:
	docker-compose up --build init-kafka
	docker-compose up --build db