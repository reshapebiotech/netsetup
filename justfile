set shell := ["bash", "-cu"]

dev:
	@echo "Starting API (mock) on :3000 and web on :5173"
	@trap 'kill 0' EXIT INT TERM; \
	MOCK_NETWORK=1 cargo run -p api & \
	cd web && bun run dev & \
	wait

test:
	cd web && bun run test:unit
	cargo test -p api --bin api

check:
	cd web && bun run check
	cargo test -p api --bin api

format:
	cargo fmt

run:
	cd web && bun run build
	MOCK_NETWORK=1 cargo run -p api

build-arm64:
	cd web && bun run build
	cargo zigbuild -p api --release --target aarch64-unknown-linux-gnu
