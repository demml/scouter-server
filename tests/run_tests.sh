docker-compose up -d
cargo test -- --test-threads=1 --nocapture
docker-compose down