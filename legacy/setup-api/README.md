# setup-api

## Developing

This project uses [`uv`](https://docs.astral.sh/uv/) to manage Python dependencies.
Once you've installed `uv` and opened this project, install dependencies with `uv sync`.

A development server can then be started with:

```sh
uv run uvicorn app.main:app --reload
```

This project uses Devcontainer for development on non-Linux systems.

If you have the _Dev Containers_ extension installed in VS Code, you can open this project in a container with the correct environment by clicking on the green icon in the bottom left corner of the window or invoking the command `Dev Containers: Open Folder [...]`.

## Testing

Testing is done using a mocked DBus connection. Tests are located in `tests/` and can be run with:

```sh
uv run pytest
```
