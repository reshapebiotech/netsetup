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

## Install on Raspberry Pi

Supported devices run 64-bit Debian/Raspberry Pi OS 12 or newer with systemd and
NetworkManager. Release bundles contain the ARM64 binary, frontend, installer,
systemd unit, and captive portal rules. Rust, Bun, and Docker are not needed on
the device.

Download a published version on an authenticated workstation or provisioning
controller. Replace the example version with the release you want:

```fish
set version v1.0.0
set bundle reshape-netsetup-$version-linux-arm64
gh release download $version --repo reshapebiotech/reshape-netsetup --pattern "$bundle.tar.gz*"
shasum -a 256 --check "$bundle.tar.gz.sha256"
scp "$bundle.tar.gz" pi@device.local:/tmp/
```

On the Pi, extract that version and prepare its configuration:

```fish
set version v1.0.0
set bundle reshape-netsetup-$version-linux-arm64
tar -xzf /tmp/$bundle.tar.gz -C /tmp
cd /tmp/$bundle
cp config.example.toml device.toml
chmod 600 device.toml
```

Edit `device.toml` to set the setup Wi-Fi password. The complete configuration is:

```toml
hostname = "ris0001"
password = "your-device-wifi-password"
```

`hostname` names the setup access point and defaults to the device's hostname.
It does not change the operating system hostname. The password must contain
8–63 printable ASCII characters. Optional `log_level` defaults to `"info"` and
also accepts `"error"`, `"warn"`, `"debug"`, and `"trace"`.

Install with:

```fish
sudo ./install.sh --config device.toml
```

The installer verifies the bundle and configuration, installs missing system
dependencies, and enables and restarts `reshape-netsetup.service`. It installs
the binary under `/opt/reshape-netsetup/` and keeps the configuration at
`/etc/reshape-netsetup/config.toml`, readable only by root. The captive portal
rules redirect AP client DNS and HTTP traffic to the application and are removed
when the service stops.

For an upgrade, extract the new bundle and run `sudo ./install.sh`. Existing
configuration is preserved. Pass `--config` only when deliberately replacing it.
An unchanged installation does not restart the service.

### Migrate an existing Docker installation

Run the new bundle's installer with the old checkout path:

```fish
sudo ./install.sh --legacy-dir /home/pi/machine-setup
```

When no installed configuration exists, the installer imports `DEVICE_NAME` and
`HOTSPOT_PASSWORD` from the old `.env`. It disables restart and stops the known
`setup-api` and `setup-ui` containers and removes the legacy self-update
cron entry. The old checkout and stopped containers remain available.

### Building releases

CI tests the API and frontend, then builds an ARM64 bundle as a downloadable
workflow artifact. Pushing a `v*` tag publishes that bundle and its SHA-256
checksum as a GitHub release after the checks pass. Tags containing a hyphen
produce prereleases.

To produce the same bundle locally:

```fish
set version v1.0.0
env RESHAPE_NETSETUP_VERSION=$version just build-arm64
./deploy/scripts/package-release.sh $version
```

Cross-compilation uses `cargo-zigbuild` with Zig as its linker, targeting glibc
2.36 for Debian 12 compatibility. Both tools are managed by mise.

## Reshape branding

Reshape copy and the support link live in `web/src/lib/branding.ts`. The logo and
favicon live in `web/static/branding/` and are embedded with the frontend in release
builds. No runtime branding configuration or separately installed assets are needed.

## Captive portal

Setup Wi-Fi intercepts DNS over UDP and TCP and directs IPv4 lookups to
`CAPTIVE_IP` (default `10.42.0.1`). IPv6 lookups return empty successful answers
so they do not invalidate the IPv4 answer.

Apple, Android, Windows, Firefox, and GNOME HTTP connectivity probes receive an
uncached redirect to the setup root, rather than an unknown frontend route.
The page and its assets are served from the device without internet access.

To verify automatic discovery, join setup Wi-Fi with a phone and wait for its
login screen before opening a browser manually. Confirm that the Reshape network
configuration screen loads. A successful HTTP probe test verifies interception;
the automatic popup itself must be checked on a real client.

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
