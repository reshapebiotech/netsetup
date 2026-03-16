from unittest import IsolatedAsyncioTestCase

import dbus
import dbusmock
import pytest
from dbusmock.templates.networkmanager import (
    MANAGER_IFACE,
    SETTINGS_IFACE,
    SETTINGS_OBJ,
    DeviceState,
    InfrastructureMode,
    NM80211ApSecurityFlags,
)
from sdbus_async.networkmanager import (
    ActiveConnection,
    NetworkDeviceWired,
    NetworkManager,
    WpaSecurityFlags,
)

from setup_api.network_manager import (
    activate_local_access_point,
    connect_to_ethernet,
    connect_to_wireless,
    delete_connection_by_uuid,
    get_applied_settings,
    get_network_devices,
    get_network_manager,
    reset_bus,
    scan_wifi,
)
from setup_api.types import Address, AllSettings, IPConfig, NetworkSettings

pytest_plugins = ("pytest_asyncio",)


class TestMyProgram(dbusmock.DBusTestCase, IsolatedAsyncioTestCase):
    @classmethod
    def setUpClass(cls):
        cls.start_system_bus()
        cls.dbus_con = cls.get_dbus(system_bus=True)

    def setUp(self):
        reset_bus()
        self.network_manager_server, self.dbus_manager = self.spawn_server_template(
            template="networkmanager",
            stdout=True,
            system_bus=True,
        )
        self.dbusmock = dbus.Interface(self.dbus_manager, dbusmock.MOCK_IFACE)
        self.settings = dbus.Interface(
            self.dbus_con.get_object(MANAGER_IFACE, SETTINGS_OBJ), SETTINGS_IFACE
        )

    def tearDown(self) -> None:
        self.network_manager_server.terminate()
        self.network_manager_server.wait()

    @pytest.mark.asyncio
    async def test_get_network_manager(self):
        network_manager = get_network_manager()
        assert isinstance(network_manager, NetworkManager)

    @pytest.mark.asyncio
    async def test_get_network_devices(self):
        self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddEthernetDevice("mock_Ethernet1", "eth0", DeviceState.ACTIVATED)
        network_manager = get_network_manager()
        connections = await get_network_devices(network_manager)
        assert isinstance(connections, list)
        assert len(connections) == 2
        interfaces = [await c.interface for c in connections]
        assert any(interface == "eth0" for interface in interfaces)
        assert any(interface == "wlan0" for interface in interfaces)

    @pytest.mark.asyncio
    async def test_get_no_network_devices(self):
        network_manager = get_network_manager()
        connections = await get_network_devices(network_manager)
        assert isinstance(connections, list)
        assert len(connections) == 0

    @pytest.mark.asyncio
    async def test_get_wireless_networks(self):
        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_WIFI",
            "SOME_WIFI",
            "00:23:F8:7E:12:BB",
            InfrastructureMode.NM_802_11_MODE_AP,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_NONE,
        )
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_OTHER_WIFI",
            "SOME_OTHER_WIFI",
            "00:23:F8:7E:12:BC",
            InfrastructureMode.NM_802_11_MODE_INFRA,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_KEY_MGMT_PSK,
        )

        network_manager = get_network_manager()
        network_devices = await get_network_devices(network_manager)
        assert len(network_devices) == 1
        interface = await network_devices[0].interface
        assert interface == "wlan0"
        networks = await scan_wifi(network_devices)
        assert len(networks) == 2
        assert networks[0].ssid == "SOME_WIFI"
        assert networks[1].ssid == "SOME_OTHER_WIFI"

    @pytest.mark.asyncio
    async def test_get_no_wireless_networks(self):
        self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        network_manager = get_network_manager()
        network_devices = await get_network_devices(network_manager)
        assert len(network_devices) == 1
        interface = await network_devices[0].interface
        assert interface == "wlan0"
        networks = await scan_wifi(network_devices)
        assert len(networks) == 0

    @pytest.mark.asyncio
    async def test_detects_network_security(self):
        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        # AddAccessPoint assumes WPA2 as a minimum, so this is a WPA2 network.
        self.dbusmock.AddAccessPoint(
            wifi,
            "WPA1_NETWORK",
            "WPA1_NETWORK",
            "00:23:F8:7E:12:DD",
            InfrastructureMode.NM_802_11_MODE_INFRA,
            2425,
            5400,
            82,
            int(WpaSecurityFlags.P2P_TKIP | WpaSecurityFlags.AUTH_PSK),
        )
        self.dbusmock.AddAccessPoint(
            wifi,
            "WPA2_NETWORK",
            "WPA2_NETWORK",
            "00:23:F8:7E:12:EE",
            InfrastructureMode.NM_802_11_MODE_INFRA,
            2425,
            5400,
            82,
            int(WpaSecurityFlags.P2P_CCMP | WpaSecurityFlags.AUTH_PSK),
        )
        self.dbusmock.AddAccessPoint(
            wifi,
            "EAP_NETWORK",
            "EAP_NETWORK",
            "00:23:F8:7E:12:FF",
            InfrastructureMode.NM_802_11_MODE_INFRA,
            2425,
            5400,
            82,
            int(WpaSecurityFlags.P2P_CCMP | WpaSecurityFlags.AUTH_802_1X),
        )
        self.dbusmock.AddAccessPoint(
            wifi,
            "WPA3_NETWORK",
            "WPA3_NETWORK",
            "00:23:F8:7E:12:AA",
            InfrastructureMode.NM_802_11_MODE_INFRA,
            2425,
            5400,
            82,
            int(WpaSecurityFlags.P2P_CCMP | WpaSecurityFlags.AUTH_SAE),
        )
        network_manager = get_network_manager()
        network_devices = await get_network_devices(network_manager)
        assert len(network_devices) == 1
        interface = await network_devices[0].interface
        assert interface == "wlan0"
        networks = await scan_wifi(network_devices)
        assert len(networks) == 4
        assert networks[0].ssid == "WPA1_NETWORK"
        assert networks[0].security == "WPA2"
        assert networks[1].ssid == "WPA2_NETWORK"
        assert networks[1].security == "WPA2"
        assert networks[2].ssid == "EAP_NETWORK"
        assert networks[2].security == "EAP"
        assert networks[3].ssid == "WPA3_NETWORK"
        assert networks[3].security == "WPA3"

    @pytest.mark.asyncio
    async def test_create_access_point(self):
        network_manager = get_network_manager()
        ssid = "MyAccessPoint"
        passphrase = "MyPassphrase"
        connection_path = await activate_local_access_point(
            ssid, passphrase, network_manager
        )
        assert isinstance(connection_path, str)
        assert connection_path.startswith(
            "/org/freedesktop/NetworkManager/ActiveConnection"
        )

    @pytest.mark.asyncio
    async def test_connect_to_access_point_with_password(self):
        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_WIFI",
            "SOME_WIFI",
            "00:23:F8:7E:12:BB",
            InfrastructureMode.NM_802_11_MODE_AP,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_PAIR_CCMP,
        )
        network_manager = get_network_manager()
        settings = NetworkSettings(ssid="MyAccessPoint", password="MyPassphrase")
        states = await connect_to_wireless(
            settings=settings, network_manager=network_manager
        )
        assert states == [("ACTIVATED", "UNKNOWN")]

    @pytest.mark.asyncio
    async def test_connect_to_access_point_with_ipv4(self):
        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_WIFI",
            "SOME_WIFI",
            "00:23:F8:7E:12:BB",
            InfrastructureMode.NM_802_11_MODE_AP,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_PAIR_CCMP,
        )
        network_manager = get_network_manager()
        settings = NetworkSettings(
            ssid="MyAccessPoint",
            password="MyPassphrase",
            ipv4=IPConfig(
                method="manual",
                address_data=[Address(address="8.8.8.8", prefix=24)],
                gateway="",
                dns=[],
            ),
        )
        states = await connect_to_wireless(
            settings=settings, network_manager=network_manager
        )
        assert states == [("ACTIVATED", "UNKNOWN")]

    @pytest.mark.asyncio
    async def test_connect_to_access_point_with_eap(self):
        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_WIFI",
            "SOME_WIFI",
            "00:23:F8:7E:12:BB",
            InfrastructureMode.NM_802_11_MODE_AP,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_KEY_MGMT_802_1X,
        )
        network_manager = get_network_manager()
        settings = NetworkSettings(
            ssid="MyAccessPoint",
            password=None,
            eap_login="myusername",
            eap_password="mypassword",
        )
        states = await connect_to_wireless(
            settings=settings, network_manager=network_manager
        )
        assert states == [("ACTIVATED", "UNKNOWN")]

    @pytest.mark.asyncio
    async def test_connect_to_access_point_with_password_and_eap(self):
        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_WIFI",
            "SOME_WIFI",
            "00:23:F8:7E:12:BB",
            InfrastructureMode.NM_802_11_MODE_AP,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_PAIR_CCMP,
        )
        network_manager = get_network_manager()
        settings = NetworkSettings(
            ssid="MyAccessPoint",
            password="MyPassphrase",
            eap_login="myusername",
            eap_password="mypassword",
        )
        states = await connect_to_wireless(
            settings=settings, network_manager=network_manager
        )
        assert states == [("ACTIVATED", "UNKNOWN")]

    @pytest.mark.asyncio
    async def test_delete_connection_by_uuid(self):
        network_manager = get_network_manager()
        ssid = "TestSSID"
        passphrase = "TestPassphrase"

        await activate_local_access_point(ssid, passphrase, network_manager)
        connections = await network_manager.active_connections
        active_connections = [
            ActiveConnection(connection) for connection in connections
        ]
        hotspot = active_connections[0]
        uuid = await hotspot.uuid

        deleted_uuid = await delete_connection_by_uuid(uuid)
        assert deleted_uuid == uuid

    @pytest.mark.asyncio
    async def test_get_applied_settings(self):
        self.dbusmock.AddEthernetDevice("eth0", "eth0", DeviceState.ACTIVATED)

        wifi = self.dbusmock.AddWiFiDevice("mock_WiFi1", "wlan0", DeviceState.ACTIVATED)
        self.dbusmock.AddAccessPoint(
            wifi,
            "SOME_WIFI",
            "SOME_WIFI",
            "00:23:F8:7E:12:BB",
            InfrastructureMode.NM_802_11_MODE_AP,
            2425,
            5400,
            82,
            NM80211ApSecurityFlags.NM_802_11_AP_SEC_PAIR_CCMP,
        )
        network_manager = get_network_manager()
        settings = NetworkSettings(
            eap_login="myusername",
            eap_password="mypassword",
            ipv4=IPConfig(
                method="manual",
                address_data=[Address(address="1.1.1.1", prefix=24)],
                gateway="",
                dns=[],
            ),
        )
        await connect_to_ethernet(settings=settings, network_manager=network_manager)

        network_devices = await get_network_devices(network_manager)
        ethernet_devices = [
            device
            for device in network_devices
            if isinstance(device, NetworkDeviceWired)
        ]

        applied_settings = await get_applied_settings(ethernet_devices)
        assert isinstance(applied_settings, AllSettings)

        settings = applied_settings.settings[0]
        assert settings.ssid == "Ethernet"
        assert settings.ipv4.method == "manual"
        assert settings.ipv4.address_data[0].address == "1.1.1.1"
        assert settings.ipv4.address_data[0].prefix == 24
        assert settings.eap_login == "myusername"

    @pytest.mark.asyncio
    async def test_connect_to_ethernet(self):
        self.dbusmock.AddEthernetDevice("mock_Ethernet1", "eth0", DeviceState.ACTIVATED)
        network_manager = get_network_manager()
        settings = NetworkSettings(ssid="Ethernet")

        states = await connect_to_ethernet(settings, network_manager)
        assert states[-1][0] == "ACTIVATED"
