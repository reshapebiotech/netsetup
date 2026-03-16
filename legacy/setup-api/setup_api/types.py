from typing import Literal

from pydantic import BaseModel

Security = Literal["WPA1", "WPA2", "WPA3", "EAP", "None"]


class Network(BaseModel):
    ssid: str
    security: Security
    strength: int


class Networks(BaseModel):
    networks: list[Network]


class Address(BaseModel):
    address: str
    prefix: int


class IPConfig(BaseModel):
    method: Literal["auto", "manual"]
    address_data: list[Address] | None = None
    gateway: str | None = None
    dns: list[str] | None = None


class NetworkSettings(BaseModel):
    ssid: str | None = None
    password: str | None = None
    id: str | None = None
    eap_login: str | None = None
    eap_password: str | None = None
    ipv4: IPConfig | None = None


class MacAddressInfo(BaseModel):
    interface_name: str  # or interface_id
    mac_address: str


class AllSettings(BaseModel):
    settings: list[NetworkSettings]
    mac_addresses: list[MacAddressInfo]


class Connectivity(BaseModel):
    is_connected: bool
    can_reach_host: bool


class AccessPointStatus(BaseModel):
    is_enabled: bool


class ConnectionAttempt(BaseModel):
    settings: NetworkSettings
    connection_result_states: list[tuple[str, str]]


class State(BaseModel):
    state: str
    wifi_settings: dict[str, str] | None = None
    connection_attempt: ConnectionAttempt | None = None


class Restart(BaseModel):
    pass
