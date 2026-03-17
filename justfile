set shell := ["bash", "-cu"]

api-run:
	MOCK_NETWORK=1 cargo run -p api

api-run-real:
	cargo run -p api

web-dev:
	cd web && bun run dev

dev:
	@echo "Run in two terminals:"
	@echo "  just api-run"
	@echo "  just web-dev"

dev-browser:
	cd web && bunx playwright test --ui

web-test:
	cd web && bun run test

web-test-unit:
	cd web && bun run test:unit

web-test-e2e:
	cd web && bun run test:e2e

check:
	cd web && bun run check
	cargo test -p api --bin api

build-api:
	cargo build -p api --release

run-api:
	cd web && bun run build
	MOCK_NETWORK=1 cargo run -p api

build-linux-arm64:
	cd web && bun run build
	cargo zigbuild -p api --release --target aarch64-unknown-linux-gnu
