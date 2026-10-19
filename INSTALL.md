# Installation

Supported devices run 64-bit Debian/Raspberry Pi OS 12 or newer with systemd and
NetworkManager. Release bundles contain the ARM64 binary, frontend, installer,
systemd unit, and captive portal rules. Rust, Bun, and Docker are not needed on
the device.

Download a release on your workstation. Replace the example version as needed:

```fish
set version v0.2.3
set bundle reshape-netsetup-$version-linux-arm64
set url https://github.com/reshapebiotech/netsetup/releases/download/$version
curl --fail --location --remote-name "$url/$bundle.tar.gz"
curl --fail --location --remote-name "$url/$bundle.tar.gz.sha256"
shasum -a 256 --check "$bundle.tar.gz.sha256"
scp "$bundle.tar.gz" pi@device.local:/tmp/
```

On the Pi, extract that version and prepare its configuration:

```fish
set version v0.2.3
set bundle reshape-netsetup-$version-linux-arm64
tar -xzf /tmp/$bundle.tar.gz -C /tmp
cd /tmp/$bundle
cp config.example.toml device.toml
chmod 600 device.toml
```

Edit `device.toml` to set the setup Wi-Fi password. The minimal configuration is:

```toml
hostname = "ris0001"
password = "your-device-wifi-password"
```

`hostname` names the setup access point and defaults to the device's hostname.
It does not change the operating system hostname. The password must contain
8–63 printable ASCII characters. Optional `log_level` defaults to `"info"` and
also accepts `"error"`, `"warn"`, `"debug"`, and `"trace"`.

Optional service checks belong in the same TOML file:

```toml
[[connectivity.groups]]
name = "Core services"
required = true
endpoints = [
  { name = "API", url = "https://api.example.com", description = "Device communication" },
  { name = "Storage", url = "https://storage.example.com", description = "Image storage" },
]

[[connectivity.groups]]
name = "Device management"
required = false
endpoints = [
  { name = "Updates", url = "https://updates.example.com", description = "Software downloads" },
]
```

Groups show full, partial, or no connectivity, with a result for each endpoint.
`required` marks services needed for normal operation; users can still explicitly
save a local connection. Saving rechecks every endpoint that passed during testing
and rolls back if any stops responding. With no groups configured, only gateway
and internet access are checked.

Endpoints must be unique HTTPS URLs, without credentials or fragments, using
IPv4 or a DNS hostname. Custom ports and paths are supported. Up to 8 groups and
16 endpoints can be configured; each group needs a name and at least one endpoint.
Checks verify DNS and TLS, then request HTTP headers without signing in or following
redirects. HTTP 200–499 responses count as reachable, including authentication
responses such as 401/403; server errors, timeouts, and DNS/TLS failures do not.
These are network-access checks, not application or permission checks.

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

### Building releases

CI tests the API and frontend, then builds an ARM64 bundle as a downloadable
workflow artifact. Pushing a `v*` tag publishes that bundle and its SHA-256
checksum as a GitHub release after the checks pass. Tags containing a hyphen
produce prereleases.

To produce the same bundle locally:

```fish
set version v0.2.3
env RESHAPE_NETSETUP_VERSION=$version just build-arm64
./deploy/scripts/package-release.sh $version
```

Cross-compilation uses `cargo-zigbuild` with Zig as its linker, targeting glibc
2.36 for Debian 12 compatibility. Both tools are managed by mise.

