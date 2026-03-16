<h1 align="center">Machine Setup (Rust Rewrite)</h1>

<p align="center">
The setup workflow for our machines.
</p>

---

An application that automates the network setup of a new machine. It exposes a Captive Portal via DNS interception, serving a Svelte 5 single-page application directly from a Rust Axum binary. It interfaces with NetworkManager on the host using `nmcli` to scan and connect to networks.

## Architecture

* **Frontend:** Svelte 5 with runes, styled via `shadcn-svelte` and TailwindCSS. Compiled into static HTML/JS/CSS.
* **Backend:** Rust API using `axum`. Statically serves the Svelte frontend and intercepts captive portal DNS queries via embedded `hickory-dns`.
* **System Integration:** `NetworkController` trait wrapping `nmcli` for real hardware, and a `MockNetworkController` for seamless local UI development on non-Linux machines.

## Local Development (No Linux/NetworkManager required)

You can run the entire stack locally on a Mac or Windows machine using the mock controller.

1. **Build the frontend:**
   ```sh
   cd web
   bun install
   bun run build
   cd ..
   ```

2. **Run the API (Mock Mode):**
   ```sh
   MOCK_NETWORK=1 cargo run -p api
   ```

3. Visit `http://localhost:3000` to interact with the UI.

Alternatively, you can run the UI via `bun run dev` inside the `web` folder, which will proxy `/api` requests to the Rust backend running on port `3000`.

## Production / End-to-End Testing

For full E2E testing mimicking the Raspberry Pi environment, you can build and run the Docker Compose setup. This spins up a debian container with system D-Bus and NetworkManager running.

```sh
docker-compose up --build
```

**Note:** The docker container runs in `--privileged` mode and `network_mode: host` to allow NetworkManager to interface with the actual kernel interfaces. 

## Testing

The API includes unit tests and property-based tests (`proptest`) for the `nmcli` parsing logic to ensure stability against weird inputs (e.g. escaped colons in SSIDs or MAC addresses). It also includes Axum API integration tests.

```sh
cargo test -p api
```
