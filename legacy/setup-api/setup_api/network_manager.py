import asyncio
import os
import socket
from typing import Sequence, Callable
from uuid import uuid4

import sdbus
from sdbus_async.networkmanager import (
    AccessPoint,
    ActiveConnection,
    ConnectionState,
    ConnectionStateReason,
    ConnectivityState,
    DeviceType,
    NetworkConnectionSettings,
    NetworkDeviceGeneric,
    NetworkDeviceWired,
    NetworkDeviceWireless,
    NetworkManager,
    NetworkManagerSettings,
    WpaSecurityFlags,
)
from sdbus_async.networkmanager.settings import (
    AddressData,
    ConnectionProfile,
    ConnectionSettings,
    EapolSettings,
    Ipv4Settings,
    WirelessSecuritySettings,
    WirelessSettings,
)

from setup_api.log import log
from setup_api.types import (
    AccessPointStatus,
    Address,
    AllSettings,
    Connectivity,
    IPConfig,
    MacAddressInfo,
    Network,
    NetworkSettings,
    Security,
)

bus: sdbus.SdBus | None = None


def get_network_manager() -> NetworkManager:
    global bus
    if not bus:
        log.debug("Opening system bus")
        bus = sdbus.sd_bus_open_system()
        sdbus.set_default_bus(bus)

    network_manager = NetworkManager(bus=bus)
    return network_manager


def reset_bus():
    global bus
    bus = None


def ping_host(host: str, timeout: float) -> bool:
    try:
        socket.setdefaulttimeout(timeout)
        socket.socket(socket.AF_INET, socket.SOCK_STREAM).connect((host.strip(), 443))
        return True
    except socket.error:
        return False


def ping(hosts: list[str], timeout: float = 3) -> bool:
    for location in hosts:
        if ping_host(location, timeout):
            return True
    return False


def map_to_names(state: ConnectionState | None, reason: ConnectionStateReason | None):
    return (
        state.name if state is not None else "",
        reason.name if reason is not None else "",
    )


async def get_network_devices(
    network_manager: NetworkManager,
) -> Sequence[NetworkDeviceWireless | NetworkDeviceWired]:
    devices = await network_manager.devices
    all_devices = {path: NetworkDeviceGeneric(path, bus=bus) for path in devices}
    wifi_devices = [
        NetworkDeviceWireless(path, bus=bus)
        for path, device in all_devices.items()
        if await device.device_type == DeviceType.WIFI
    ]

    ethernet_devices = [
        NetworkDeviceWired(path, bus=bus)
        for path, device in all_devices.items()
        if await device.device_type == DeviceType.ETHERNET
    ]

    all_devices = wifi_devices + ethernet_devices

    return all_devices


async def scan_wifi(devices: Sequence[NetworkDeviceWireless]) -> list[Network]:
    hotspot_ssid = os.environ.get("DEVICE_NAME", "Reshape Device")
    networks: list[Network] = []
    for device in devices:
        interface = None
        try:
            interface = await device.interface
            log.debug(f"Scanning '{interface}' for access points")
            await device.request_scan({})
            access_points = await device.get_all_access_points()
            for ap_path in access_points:
                access_point = AccessPoint(ap_path, bus=bus)
                ssid_bytes = await access_point.ssid
                ssid = ssid_bytes.decode("utf-8")
                if not ssid or ssid == "" or ssid == hotspot_ssid:
                    continue

                strength = await access_point.strength
                wpa_flags = WpaSecurityFlags(await access_point.wpa_flags)
                rsn_flags = WpaSecurityFlags(await access_point.rsn_flags)

                networks.append(
                    Network(
                        ssid=ssid,
                        security=determine_network_security(
                            wpa_flags=wpa_flags, rsn_flags=rsn_flags
                        ),
                        strength=strength,
                    )
                )
        except Exception as e:
            log.info(f"Error scanning device {interface}: {e}")
            continue

    return networks


async def scan_ethernet(devices: Sequence[NetworkDeviceWired]) -> list[Network]:
    networks: list[Network] = []
    for device in devices:
        interface = None
        try:
            interface = await device.interface
            active_connection = await device.active_connection
            log.debug(f"Saw an active connection for '{interface}'")

            if not active_connection:
                continue

            uuid = await ActiveConnection(active_connection, bus=bus).uuid
            settings = await NetworkManagerSettings(bus=bus).get_connection_by_uuid(
                uuid
            )
            profile = await NetworkConnectionSettings(settings, bus=bus).get_profile()
            settings = eap = profile.to_settings_dict()
            eap = settings.get("802-1x", None)
            networks.append(
                Network(
                    ssid="Ethernet",
                    security="EAP" if eap else "None",
                    strength=100,
                )
            )

        except Exception as e:
            log.error(f"Error scanning device {interface}: {e}")
            continue

    return networks


async def profile_to_settings(profile: ConnectionProfile) -> NetworkSettings:
    settings = NetworkSettings()
    settings.id = profile.connection.uuid
    settings.ssid = str(profile.wireless.ssid) if profile.wireless else "Ethernet"
    settings.eap_login = profile.eapol.identity if profile.eapol else None
    settings.eap_password = profile.eapol.password if profile.eapol else None

    settings.ipv4 = IPConfig(
        method=profile.ipv4.method,
        address_data=[
            Address(address=address.address, prefix=address.prefix)
            for address in profile.ipv4.address_data
        ]
        if profile.ipv4.address_data
        else None,
        gateway=profile.ipv4.gateway,
        dns=profile.ipv4.dns_data,
    )

    return settings


async def get_applied_settings(devices: Sequence[NetworkDeviceGeneric]) -> AllSettings:
    hotspot_ssid = os.environ.get("DEVICE_NAME", "Reshape Device")
    profiles: list[ConnectionProfile] = []
    mac_addresses = []
    for device in devices:
        interface = None
        try:
            interface = await device.interface
            connection = await device.active_connection
            connection = ActiveConnection(connection, bus=bus)
            uuid = await connection.uuid
            id = await connection.id
            log.debug(f"Saw connection with UUID: {uuid}")
            if id == hotspot_ssid:
                continue

            settings = await NetworkManagerSettings(bus=bus).get_connection_by_uuid(
                uuid
            )
            profile = await NetworkConnectionSettings(settings, bus=bus).get_profile(
                fetch_secrets=True
            )
            profiles.append(profile)
            hardware_address = await device.hw_address
            mac_addresses.append(
                MacAddressInfo(interface_name=interface, mac_address=hardware_address)
            )
        except Exception as e:
            log.info(f"Error getting settings for device {interface}: {e}")
            continue

    mapped = [await profile_to_settings(profile) for profile in profiles]
    return AllSettings(settings=mapped, mac_addresses=mac_addresses)


async def delete_connection_by_uuid(uuid: str) -> str:
    await NetworkManagerSettings(bus=bus).delete_connection_by_uuid(uuid)
    return uuid


def build_ip_settings(settings: NetworkSettings) -> Ipv4Settings:
    return (
        Ipv4Settings(method="auto")
        if not settings.ipv4
        else Ipv4Settings(
            method=settings.ipv4.method if settings.ipv4.method else "auto",
            address_data=[
                AddressData(address=settings.address, prefix=settings.prefix)
                for settings in settings.ipv4.address_data or []
            ],
            gateway=settings.ipv4.gateway if settings.ipv4.gateway else None,
            dns_data=settings.ipv4.dns if settings.ipv4.dns else None,
        )
    )


async def connect_to_wireless(
    settings: NetworkSettings,
    network_manager: NetworkManager,
) -> list[tuple[str, str]]:
    ssid = settings.ssid if settings.ssid else "Wireless Network"
    password = settings.password
    id = str(settings.id) if settings.id else str(uuid4())
    eap_login = settings.eap_login
    eap_password = settings.eap_password

    ipv4 = build_ip_settings(settings=settings)

    profile = ConnectionProfile(
        connection=ConnectionSettings(
            uuid=id,
            connection_type="802-11-wireless",
            connection_id=ssid,
            autoconnect=True,
        ),
        wireless=WirelessSettings(ssid=ssid.encode("utf-8")),
        ipv4=ipv4,
    )

    if password:
        profile.wireless_security = WirelessSecuritySettings(
            key_mgmt="wpa-psk" if not (eap_login and eap_password) else "wpa-eap",
            psk=password,
            auth_alg="open",
        )

    if eap_login and eap_password:
        profile.eapol = EapolSettings(
            eap=["peap"],
            identity=eap_login,
            password=eap_password,
        )

    try:
        mapped = await connect_with_settings(network_manager, profile)
        log.debug(f"Connection to {ssid} completed with statuses: {mapped}")
    except Exception as e:
        log.error(f"Error connecting to {ssid}: {e}")
        return []
    return mapped


async def connect_to_ethernet(
    settings: NetworkSettings,
    network_manager: NetworkManager,
) -> list[tuple[str, str]]:
    id = str(settings.id) if settings.id else str(uuid4())
    eap_login = settings.eap_login
    eap_password = settings.eap_password

    ipv4 = build_ip_settings(settings=settings)

    profile = ConnectionProfile(
        connection=ConnectionSettings(
            uuid=str(id),
            connection_type="802-3-ethernet",
            connection_id="Ethernet",
            autoconnect=True,
        ),
        ipv4=ipv4,
    )

    if eap_login and eap_password:
        profile.eapol = EapolSettings(
            eap=["peap"],
            identity=eap_login,
            password=eap_password,
        )

    try:
        mapped = await connect_with_settings(network_manager, profile)
        log.debug(f"Connection to Ethernet completed with statuses: {mapped}")
    except Exception as e:
        log.error(f"Error connecting to Ethernet: {e}")
        return []

    return mapped


async def connect_with_settings(network_manager, profile):
    added = await NetworkManagerSettings(bus=bus).add_connection(profile.to_dbus())
    connection_path = await network_manager.activate_connection(added, "/", "/")
    active_connection = ActiveConnection(connection_path, bus=bus)

    finished_states = {ConnectionState.ACTIVATED, ConnectionState.DEACTIVATED}
    initial_state = await active_connection.state
    states: list[tuple[ConnectionState, ConnectionStateReason]] = [
        (ConnectionState(initial_state), ConnectionStateReason.UNKNOWN)
    ]
    if not initial_state == ConnectionState.ACTIVATED:
        async for state, reason in active_connection.state_changed:
            state_mapped, reason_mapped = (
                ConnectionState(state),
                ConnectionStateReason(reason),
            )
            log.debug(
                f"State, reason changed to {map_to_names(state_mapped, reason_mapped)}"
            )
            states.append((state_mapped, reason_mapped))
            if state_mapped in finished_states:
                break

    (last_state, _) = states[-1]
    if last_state != ConnectionState.ACTIVATED:
        await delete_connection_by_uuid(await active_connection.uuid)

    unique = list(dict.fromkeys(states))
    mapped = [map_to_names(s, r) for s, r in unique]
    return mapped


def determine_network_security(
    wpa_flags: WpaSecurityFlags, rsn_flags: WpaSecurityFlags
) -> Security:
    if rsn_flags & WpaSecurityFlags.AUTH_SAE:
        return "WPA3"
    if rsn_flags & WpaSecurityFlags.AUTH_PSK:
        return "WPA2"
    if rsn_flags & WpaSecurityFlags.AUTH_802_1X:
        return "EAP"
    if wpa_flags & WpaSecurityFlags.AUTH_PSK:
        return "WPA1"
    if wpa_flags & WpaSecurityFlags.AUTH_802_1X:
        return "EAP"

    return "None"


async def activate_local_access_point(
    ssid: str,
    passphrase: str,
    network_manager: NetworkManager,
) -> str | None:
    profile = ConnectionProfile(
        connection=ConnectionSettings(
            connection_type="802-11-wireless",
            connection_id=ssid,
            autoconnect=True,
        ),
        wireless=WirelessSettings(
            ssid=ssid.encode("utf-8"),
            mode="ap",
            band="bg",
            channel=1,
        ),
        wireless_security=WirelessSecuritySettings(key_mgmt="wpa-psk", psk=passphrase),
        ipv4=Ipv4Settings(method="shared"),
    )

    connections = await network_manager.active_connections
    active_connections = [ActiveConnection(connection) for connection in connections]
    hotspots = [
        connection for connection in active_connections if (await connection.id) == ssid
    ]

    log.debug(f"Currently active connections: {connections}")
    log.debug(f"Currently active hotspots: {len(hotspots)}")
    connection_path = None
    if not hotspots:
        log.info("Creating local access point")
        created = await NetworkManagerSettings(bus=bus).add_connection(
            profile.to_dbus()
        )
        connection_path = await network_manager.activate_connection(created, "/", "/")
        log.info(f"Created local access point: '{connection_path}'")
        return connection_path

    log.debug("Local access point is already active")
    return connection_path


async def deactivate_local_access_point(
    ssid: str,
    network_manager: NetworkManager,
) -> str | None:
    connections = await network_manager.active_connections
    active_connections = [
        ActiveConnection(connection, bus=bus) for connection in connections
    ]
    hotspots = [
        connection for connection in active_connections if (await connection.id) == ssid
    ]
    if not hotspots:
        log.debug("No local access point to deactivate")
        return None

    hotspot = hotspots[0]
    state = ConnectionState(await hotspot.state)
    if state == ConnectionState.ACTIVATED:
        log.info("Deactivating the local access point")
        uuid = await hotspot.uuid
        connection_path = await hotspot.connection
        log.info(f"Deleting connection '{connection_path}' with UUID: {uuid}")
        await NetworkManagerSettings(bus=bus).delete_connection_by_uuid(uuid)

    return await hotspot.connection


async def access_point_status(
    ssid: str, network_manager: NetworkManager
) -> AccessPointStatus:
    connections = await network_manager.active_connections
    active_connections = [
        ActiveConnection(connection, bus=bus) for connection in connections
    ]
    hotspots = [
        connection for connection in active_connections if (await connection.id) == ssid
    ]

    return AccessPointStatus(is_enabled=hotspots != [])


async def get_connectivity(network_manager: NetworkManager):
    ping_locations = os.environ.get(
        "SERVER_PING_LOCATION", "reshapebiotech.com, ris-api.reshapebiotech.com"
    ).split(sep=",")
    connectivity = await network_manager.connectivity
    ping_result = ping(ping_locations)
    return Connectivity(
        can_reach_host=ping_result,
        is_connected=(connectivity == ConnectivityState.FULL),
    )


async def determine_hotspot_status(has_access_point_activity: Callable[..., bool]):
    ssid = os.environ.get("DEVICE_NAME", "Reshape Device")
    passphrase = os.environ.get("HOTSPOT_PASSWORD", "setup-device")
    ping_locations = os.environ.get(
        "SERVER_PING_LOCATION", "reshapebiotech.com, api.reshapebiotech.com"
    ).split(sep=",")
    successful_attempts = 0
    while True:
        try:
            network_manager = get_network_manager()
            ping_result = ping(ping_locations)
            if ping_result or not has_access_point_activity():
                log.info("Internet connectivity is available")
                await deactivate_local_access_point(ssid, network_manager)
                successful_attempts = min(successful_attempts + 1, 3)
            elif not ping_result:
                successful_attempts = max(successful_attempts - 1, 0)
                if successful_attempts == 0:
                    log.info("No internet connectivity!")
                    await activate_local_access_point(ssid, passphrase, network_manager)
        except Exception as e:
            log.error(f"Error in hotspot status check: {e}")
        finally:
            await asyncio.sleep(20)
