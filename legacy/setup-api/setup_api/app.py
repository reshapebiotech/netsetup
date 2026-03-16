import asyncio
import json
import os
from contextlib import asynccontextmanager
import time
from typing import Callable

import uvicorn
from fastapi import FastAPI, HTTPException, Request, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi_cache import FastAPICache
from fastapi_cache.backends.inmemory import InMemoryBackend
from fastapi_cache.decorator import cache

from setup_api.log import log
from setup_api.network_manager import (
    DeviceType,
    access_point_status,
    connect_to_ethernet,
    connect_to_wireless,
    delete_connection_by_uuid,
    determine_hotspot_status,
    get_applied_settings,
    get_connectivity,
    get_network_devices,
    get_network_manager,
    scan_ethernet,
    scan_wifi,
)
from setup_api.types import (
    AccessPointStatus,
    AllSettings,
    ConnectionAttempt,
    Connectivity,
    Networks,
    NetworkSettings,
    State,
)

STATE_LOCATION = "/tmp/state.json"
ACTIVITY_THRESHOLD = 300

global last_request_time


def _is_active() -> bool:
    """
    Determine if the server is active.
    If the server has been idle for more than ACTIVITY_THRESHOLD seconds, return False, then reset the last_request_time
    """
    global last_request_time
    delta = time.time() - last_request_time
    if delta < ACTIVITY_THRESHOLD:
        return True
    # If the server has been idle for more than ACTIVITY_THRESHOLD seconds, return False
    # This is to trigger the hotspot status check
    last_request_time = time.time()
    return False


@asynccontextmanager
async def lifespan(_: FastAPI):
    global last_request_time
    last_request_time = time.time()

    FastAPICache.init(InMemoryBackend(), prefix="fastapi-cache")
    task = asyncio.create_task(determine_hotspot_status(_is_active))
    yield
    task.cancel()


app = FastAPI(lifespan=lifespan)


app.add_middleware(
    CORSMiddleware,
    allow_credentials=True,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.middleware("http")
async def save_last_request_time(
    request: Request, call_next: Callable[..., Response]
) -> Response:
    global last_request_time
    last_request_time = time.time()
    response = await call_next(request)
    return response


def get_state_from_disk() -> State:
    with open(STATE_LOCATION, "r") as f:
        contents = f.read()
        try:
            raw = json.loads(contents)
            loaded = State(**raw)
            return loaded
        except Exception:
            log.error("Failed to decode state file, returning default state")
            log.error(f"Read: '{contents}'")
            return State(state="/")


@app.get("/state")
def get_state() -> State:
    return get_state_from_disk()


@app.post("/state")
def set_state(state: State) -> State:
    with open(STATE_LOCATION, "wb") as f:
        dumped = state.model_dump_json()
        f.write(dumped.encode("utf-8"))

    return state


@app.get("/wireless-networks")
async def available_wireless_networks() -> Networks:
    network_manager = get_network_manager()
    all_devices = await get_network_devices(network_manager=network_manager)
    wifi_devices = [d for d in all_devices if (await d.device_type) == DeviceType.WIFI]
    wlan0 = [d for d in wifi_devices if (await d.interface) == "wlan0"]

    if not wlan0:
        raise Exception("Failed to find any WiFi devices")

    wifi_networks = await scan_wifi(wlan0)
    return Networks(networks=wifi_networks)


@app.get("/wired-networks")
async def available_wired_networks() -> Networks:
    network_manager = get_network_manager()
    all_devices = await get_network_devices(network_manager=network_manager)
    ethernet_devices = [
        d for d in all_devices if (await d.device_type) == DeviceType.ETHERNET
    ]
    eth0 = [d for d in ethernet_devices if (await d.interface) == "eth0"]

    if not eth0:
        raise Exception("Failed to find any Ethernet devices")

    ethernet_networks = await scan_ethernet(eth0)
    return Networks(networks=ethernet_networks)


@app.post("/wireless-settings")
async def set_wireless_settings(settings: NetworkSettings) -> ConnectionAttempt:
    network_manager = get_network_manager()
    all_devices = await get_network_devices(network_manager=network_manager)
    wifi_devices = [d for d in all_devices if (await d.device_type) == DeviceType.WIFI]
    wlan0 = [d for d in wifi_devices if (await d.interface) == "wlan0"]

    if not wlan0:
        raise Exception("Failed to find any WiFi devices")

    current_state = get_state_from_disk()
    del current_state.connection_attempt
    set_state(current_state)

    state_reasons = await connect_to_wireless(
        settings=settings, network_manager=network_manager
    )

    if not state_reasons:
        raise HTTPException(status_code=422, detail="Failed to connect to WiFi network")

    attempt = ConnectionAttempt(
        settings=settings, connection_result_states=state_reasons
    )

    current_state = get_state_from_disk()
    current_state.connection_attempt = attempt
    set_state(current_state)

    return attempt


@app.post("/ethernet-settings")
async def ethernet_settings(settings: NetworkSettings) -> ConnectionAttempt:
    network_manager = get_network_manager()
    all_devices = await get_network_devices(network_manager=network_manager)
    ethernet_devices = [
        d for d in all_devices if (await d.device_type) == DeviceType.ETHERNET
    ]
    eth0 = [d for d in ethernet_devices if (await d.interface) == "eth0"]

    if not eth0:
        raise Exception("Failed to find any Ethernet devices")

    current_state = get_state_from_disk()
    del current_state.connection_attempt
    set_state(current_state)

    state_reasons = await connect_to_ethernet(
        settings=settings, network_manager=network_manager
    )

    if not state_reasons:
        raise HTTPException(
            status_code=422, detail="Failed to connect to Ethernet network"
        )

    attempt = ConnectionAttempt(
        settings=settings, connection_result_states=state_reasons
    )

    current_state = get_state_from_disk()
    current_state.connection_attempt = attempt
    set_state(current_state)

    return attempt


@app.get("/network-settings")
async def get_network_settings() -> AllSettings:
    network_manager = get_network_manager()
    all_devices = await get_network_devices(network_manager=network_manager)
    settings = await get_applied_settings(devices=all_devices)

    return settings


@app.delete("/network-settings/{id}")
async def delete_connection_settings(id: str) -> str:
    deleted = await delete_connection_by_uuid(uuid=id)
    return deleted


@app.get("/connectivity")
@cache(expire=60)
async def connectivity() -> Connectivity:
    network_manager = get_network_manager()
    connectivity = await get_connectivity(network_manager=network_manager)
    return connectivity


@app.get("/")
async def root() -> str:
    return "OK"


@app.get("/access_point")
async def get_access_point_status() -> AccessPointStatus:
    ssid = os.environ.get("DEVICE_NAME", "Reshape Device")

    network_manager = get_network_manager()
    status = await access_point_status(ssid=ssid, network_manager=network_manager)
    return status


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=9000)
