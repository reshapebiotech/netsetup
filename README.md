<h1 align="center">Machine Setup</h1>

<p align="center">
<img height="256" src="docs/icon.png">
</p>

<p align="center">The setup workflow for our machines.</p>

---

An application that automates the network setup of a new machine. When the application detects that the machine is not connected to a network, it will automatically create an access point for the user to connect to. The user can then configure the network settings of the machine through a web interface.

## Development

The application relies heavily on [NetworkManager](https://en.wikipedia.org/wiki/NetworkManager) to configure active network settings and therefore expects to be run on a Linux system where [`systemd`](https://en.wikipedia.org/wiki/Systemd)/[D-Bus](https://en.wikipedia.org/wiki/D-Bus) is available. Configurations to allow developing the application on non-Linux systems without these services are provided.

---

### Compose

A full development environment can be set up using Compose. The `docker-compose.yml` defines the services required to run the application. These services include the application itself, a simulated NetworkManager, a D-Bus instance (required for communicating with NetworkManager), and the web UI.

Multiple profiles are available depending on the development needs:

- `api-simulated`: The development profile for simulating required Linux services - this runs the API and all required services in a simulated Linux environment. This will only start the API and simulate an active NetworkManager and D-Bus instance. This is useful for developing the UI in isolation or on non-Linux systems.
- `all-simulated`: The development profile for simulating required Linux services - this runs both the API and UI and all required services in a simulated Linux environment. This will start all services and simulate an active NetworkManager and D-Bus instance. This is useful for end-to-end validation on non-Linux systems before releasing the project to production Linux systems.
- `api`: The API only profile - this is useful when developing the API on Linux systems.
- `""`: The default profile - this is useful for starting the API and the UI on Linux systems.

The default profile is selected when running `docker compose up`.

---

Compose services are started with:

```sh
docker compose --profile <profile> up
```

Compose Watch configurations are set up for the API and UI services. These configurations allow the services to be restarted when the source code changes. The API and UI services can be started with:

```sh
docker compose --profile <profile> up --watch
```

All services can be rebuilt with the `--build` flag:

```sh
docker compose --profile <profile> up --build
```

---

### UI development

The UI can be developed independently of the API. The UI is a Svelte application that can be run using the following commands:

```sh
bun run dev
```

Note that the UI relies on the API to be fetch network connection data as expected. The API can be started using the Compose configuration detailed above.

The UI project can be started separately using the instuctions in the `setup-ui` [README](setup-ui/README.md).

---

### API development

The API can be developed independently of the UI and without the required underlying Linux services. The API is a Python FastAPI application that can be run using the following commands:

```sh
uv run uvicorn app.main:app
```

A VSCode Devcontainer is provided to develop the API on both Linux and non-Linux systems. The _Dev Containers_ extension must be installed in VS Code. The Devcontainer can be started either when the `setup-api` project is opened in the editor or via the green icon in the bottom left corner of the window or by invoking the command `Dev Containers: Open Folder [...]`.

On Linux systems, the API can be started using the instructions in the `setup-api` [README](setup-api/README.md).
