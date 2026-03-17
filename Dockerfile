FROM oven/bun:1 AS web-builder

WORKDIR /usr/src/app/web
COPY web/package.json ./
COPY web/svelte.config.js ./
COPY web/vite.config.ts ./
COPY web/tsconfig.json ./
COPY web/postcss.config.cjs ./
COPY web/tailwind.config.ts ./
RUN bun install
COPY web/ ./
RUN bun run build

FROM rust:bookworm AS builder

WORKDIR /usr/src/app
COPY . .
COPY --from=web-builder /usr/src/app/web/build /usr/src/app/web/build

# Build API
RUN cargo build --release -p api

FROM debian:bookworm-slim

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    network-manager \
    dbus \
    iproute2 \
    wireless-tools \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create D-Bus directories
RUN mkdir -p /var/run/dbus /run/dbus

COPY --from=builder /usr/src/app/target/release/api /usr/local/bin/api
COPY --from=builder /usr/src/app/web/build /usr/share/machine-setup/web/build

ENV STATIC_DIR=/usr/share/machine-setup/web/build
ENV MOCK_NETWORK=0
# Port config for DNS 
ENV DNS_PORT=53

# Basic entrypoint to start dbus, network-manager, and our API
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 3000 53/udp

ENTRYPOINT ["/entrypoint.sh"]
