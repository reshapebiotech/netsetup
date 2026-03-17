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
   mise run api
   ```

4. **Run web dev server:**
   ```sh
   mise run web
   ```

5. Visit `http://localhost:5173`.

## Production / End-to-End Testing

The root `Dockerfile` builds web assets before compiling the API, so release builds embed the UI into the API binary. The runtime image still includes `web/build`, and `STATIC_DIR` can override serving from disk if needed.

For full E2E testing mimicking Raspberry Pi setup:

```sh
docker buildx bake -f docker-bake.hcl machine-setup-test
```

Integration tests run the image in privileged mode with NetworkManager and D-Bus.

## Testing

Run API tests:

```sh
cargo test -p api
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
