set shell := ["bash", "-cu"]

dev:
	@echo "Starting API (mock) on :3000 and web on :5173"
	@trap 'kill 0' EXIT INT TERM; \
	MOCK_NETWORK=1 cargo run -p api --bin reshape-netsetup & \
	cd web && bun run dev & \
	wait

test:
	cd web && bun run test:unit
	cargo nextest run -p api --bin reshape-netsetup

check:
	cd web && bun run check
	cargo nextest run -p api --bin reshape-netsetup

format:
	cargo fmt

run:
	cd web && bun run build
	MOCK_NETWORK=1 cargo run -p api --bin reshape-netsetup

build-arm64:
	cd web && bun run build
	cargo zigbuild -p api --bin reshape-netsetup --release --locked --target aarch64-unknown-linux-gnu.2.36
