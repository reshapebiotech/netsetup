<h1 align="center">Machine Setup (Rust Rewrite)</h1>

<p align="center">
The setup workflow for our machines.
</p>

---

An application that automates network setup of a new machine. It exposes a Captive Portal via DNS interception, serves a SvelteKit frontend from a Rust Axum binary, and interfaces with NetworkManager via `nmcli`.

## Architecture

* **Frontend:** SvelteKit + Tailwind + shadcn-style components in `web/`, compiled to `web/build`.
* **Backend:** Rust API using `axum`. In release builds it serves embedded frontend assets from the binary; in dev it can serve from filesystem fallback.
* **System Integration:** `NetworkController` trait wrapping `nmcli` for real hardware, and a `MockNetworkController` for seamless local UI development on non-Linux machines.

## Local Development (No Linux/NetworkManager required)

You can run the stack locally with the mock controller.

1. **Install tools (once):**
   ```sh
   mise install
   ```

2. **Install web dependencies:**
   ```sh
   cd web && bun install
   ```

3. **Run API (mock mode):**
   ```sh
   just api-run
   ```

4. **Run web dev server:**
   ```sh
   just web-dev
   ```

5. Visit `http://localhost:5173`.

## Testing

Run API tests:

```sh
cargo test -p api --bin api
```

Run web checks + unit tests:

```sh
cd web
bun run check
bun run test:unit
```

Run browser E2E (headless, using local Chromium on macOS):

```sh
PLAYWRIGHT_CHROMIUM_PATH=/Applications/Chromium.app/Contents/MacOS/Chromium bun run test:e2e
```

## Build + Run API

Single command that builds web assets and runs the Rust API (mock network):

```sh
just run-api
```

## Cross Compile for Raspberry Pi (Linux ARM64)

Build web assets and cross-compile the API binary with `cargo-zigbuild`:

```sh
just build-linux-arm64
```
