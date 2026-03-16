FROM python:3.11-slim
COPY --from=ghcr.io/astral-sh/uv:latest /uv /bin/uv

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    libdbus-1-dev \
    libkrb5-dev \
    libssl-dev \
    libffi-dev \
    libglib2.0-dev \
    pkg-config \
    libsystemd-dev \
    python3 \
    python3-dev \
    dbus \
    && rm -rf /var/lib/apt/lists/*


WORKDIR /usr/src/app

RUN echo '{"state": "/"}' > /tmp/state.json

COPY uv.lock pyproject.toml ./
RUN uv sync --no-dev

COPY setup_api/ setup_api/

EXPOSE 9000
ENV HOST=0.0.0.0
ENV DBUS_SYSTEM_BUS_ADDRESS=unix:path=/var/run/dbus/system_bus_socket

ENV DEVICE_NAME="Reshape Device"
ENV HOTSPOT_PASSWORD="setup-device"
ENV SERVER_PING_LOCATION="reshapebiotech.com,ris-api.reshapebiotech.com"
ENV DBUS_SYSTEM_BUS_ADDRESS="unix:path=/var/run/dbus/system_bus_socket"

ENTRYPOINT ["uv", "run", "uvicorn", "setup_api.app:app", "--host", "0.0.0.0", "--port", "9000"]
