<h1 align="center">reshape-netsetup</h1>

<p align="center">
Device network setup with captive portal support.
</p>

---

An application that automates first-run network setup. It exposes a captive portal via DNS interception, serves a SvelteKit frontend from a Rust Axum binary, and interfaces with NetworkManager via `nmcli`.

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

3. **Run API + web (mock mode):**
   ```sh
   just dev
   ```

4. Visit `http://localhost:5173`.

## Testing

Run API tests:

```sh
cargo test -p api --bin reshape-netsetup
```

Run deeper local property-test sessions:

```sh
PROPTEST_CASES=5000 PROPTEST_MAX_SHRINK_ITERS=20000 cargo test -p api --bin reshape-netsetup network::ap_supervisor -- --nocapture
```

Regression seeds for proptest are kept in `proptest-regressions/`.

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
just run
```

## Cross Compile for Raspberry Pi (Linux ARM64)

Build web assets and cross-compile the API binary with `cargo-zigbuild`:

```sh
just build-arm64
```

## Raspberry Pi Captive Portal Setup

This repo includes production helpers for running as a captive portal service on Raspberry Pi:

- `deploy/systemd/reshape-netsetup.service`
- `deploy/scripts/captive-portal-rules.sh`
- `deploy/scripts/install-rpi.sh`

### What the rules do

- Redirect DNS traffic from AP clients on `wlan0` to the app DNS port (`10053`)
- Redirect HTTP traffic from AP clients on port `80` to the app port (`3000`)
- Enable forwarding/NAT for AP clients
- Automatically remove the rules when the service stops

### Install on Pi

1. Copy the built binary + repo deploy files to the Pi.
2. Run:

```sh
./deploy/scripts/install-rpi.sh /path/to/reshape-netsetup-binary
```

If your binary is already at `./target/aarch64-unknown-linux-gnu/release/reshape-netsetup`, the argument can be omitted.

### Service configuration

The default unit uses:

- `AP_IFACE=wlan0`
- `DNS_PORT=10053`
- `HTTP_PORT=80`
- `APP_PORT=3000`
- `CAPTIVE_IP=10.42.0.1`
- `CONNECTIVITY_PROBE_TARGET=1.1.1.1` (host or URL used for internet reachability checks, e.g. `https://ris-api.reshapebiotech.com`)

Adjust in `deploy/systemd/reshape-netsetup.service` if needed, then:

```sh
sudo systemctl daemon-reload
sudo systemctl restart reshape-netsetup
```

## Branding configuration

Branding is loaded by the API from TOML and exposed at `GET /api/branding`.

- Default path: `/etc/reshape-netsetup/branding.toml`
- Override with `BRANDING_CONFIG_PATH=/custom/path.toml`
- Branding assets are served from the branding TOML directory by default
- Optional override: `BRANDING_ASSETS_DIR=/custom/assets/dir`
- Missing/invalid file falls back to built-in defaults
- Starter template: `deploy/branding.example.toml`
- Local dev profile used by `just dev`: `deploy/branding.reshape.toml`
- Local dev assets are in `deploy/logo.png` and `deploy/favicon.ico`

Example:

```toml
product_name = "Reshape Netsetup"
company_name = "Reshape Biotech"
headline = "Network configuration"
support_text = "Connect this device to your network."
success_message = "Your device is now online and ready to use."
logo_path = "/branding/logo.png"
background_path = ""
favicon_path = "/branding/favicon.ico"
help_title = "Need help?"
help_text = "Use the resources below for setup guidance."

[[help_links]]
label = "Setup guide"
url = "https://example.com/setup-guide"
```
