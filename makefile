PROJECT=scouter-server
SOURCE_OBJECTS=src

.PHONY: build
format:
	cargo fmt

.PHONY: lints
lints:
	cargo clippy --workspace --all-targets --features "rabbitmq,kafka" -- -D warnings

.PHONY: test
test:
	cargo test --features rabbitmq,kafka -- --nocapture  --test-threads=1

.PHONY: test.ignored
test.ignored:
	cargo test --features rabbitmq,kafka -- --nocapture  --test-threads=1 --ignored

.PHONY: setup.local
setup.local:
	docker-compose down
	docker-compose up --build init-kafka
	docker-compose up --build db