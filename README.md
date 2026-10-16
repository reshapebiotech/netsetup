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

Development uses Node 24 LTS and Bun from `mise.toml`. TypeScript stays on the
latest 6.x release supported by SvelteKit and `svelte-check`.

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
cargo nextest run -p api --bin reshape-netsetup
```

Run deeper local property-test sessions:

```sh
env PROPTEST_CASES=5000 PROPTEST_MAX_SHRINK_ITERS=20000 cargo nextest run -p api --bin reshape-netsetup -E 'test(network::ap_supervisor)'
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
env PLAYWRIGHT_CHROMIUM_PATH=/Applications/Chromium.app/Contents/MacOS/Chromium bun run test:e2e
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

## Reshape branding

Reshape copy and the support link live in `web/src/lib/branding.ts`. The logo and
favicon live in `web/static/branding/` and are embedded with the frontend in release
builds. No runtime branding configuration or separately installed assets are needed.

## Connection testing

Tests use NetworkManager checkpoints through its system D-Bus API, accessed with
`busctl`. The target device needs NetworkManager with checkpoint support,
`busctl` with JSON output, `curl`, `ping`, `iptables`, and a working CA certificate bundle.
The installer checks the required commands and source files before making changes.
It atomically replaces the binary and service files, then explicitly restarts the
service, so the same command supports upgrades while the app is running.

Only one network-changing operation runs at a time. Each test creates a unique
profile with autoconnect disabled and a 120-second rollback deadline. Existing
profiles are preserved. Tests restore the previous connection even when successful,
then publish their result so the user can reconnect to setup Wi-Fi and review it.
NetworkManager also rolls back if the application stops during the test.

The result distinguishes:

- Network connection and a usable IPv4 address.
- Gateway response to an interface-bound ping, or no configured gateway.
- Internet access: an HTTPS request to
  `https://connectivitycheck.gstatic.com/generate_204` returns an empty HTTP 204.
- Reshape reachability: an authenticated HTTPS response below HTTP 500 from
  `https://ris-api.reshapebiotech.com`. Authentication errors and a missing root
  route still prove that the service is reachable.

DNS queries use the tested interface and its IPv4 DNS servers; HTTPS and gateway
probes are also bound to that interface. Another uplink cannot make these probes
pass. Blocked probes are shown as unverified rather than proof of an outage.
The checks currently cover IPv4; DNS responses requiring TCP fallback remain unverified.

Wi-Fi uses `AP_IFACE` (default `wlan0`) when available. Ethernet testing requires
one Ethernet interface; ambiguous interface selection is rejected.

A locally connected network can be saved with **Save anyway**. Save applies a new
profile under another checkpoint and repeats the checks. It commits only if the
checks that succeeded during the test still succeed, then removes the superseded
managed profile. Failure restores the previous connection. The UI polls the actual
save state; a lost response is never presented as confirmed success.

## Boot access point policy

Automatic AP startup is limited to one decision per device boot. The default
`AP_BOOT_GRACE_SECONDS=90` allows existing networks to connect first. If internet
is observed, the decision is complete; otherwise the AP is enabled after the grace
period. Later internet outages do not enable it.

The service starts after NetworkManager without waiting for `network-online.target`.
It uses device uptime, and stores completed decisions under
`/run/reshape-netsetup/boot-decision` so service restarts do not repeat the decision.
The supplied systemd unit preserves this directory across service restarts; reboot
clears it. Starting the service after the boot grace period skips automatic startup.

Manual AP activation, resetting network settings, and checkpoint restoration during
configuration are deliberate exceptions to boot-only automatic startup.
