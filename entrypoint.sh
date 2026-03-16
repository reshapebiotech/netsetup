#!/bin/bash
set -e

# Start D-Bus daemon
dbus-daemon --system --fork

# Start NetworkManager in the background
NetworkManager &

# Wait for NM to come up
sleep 2

# Start the application
exec /usr/local/bin/api
