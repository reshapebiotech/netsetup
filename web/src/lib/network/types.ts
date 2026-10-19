export type Transport = 'wifi' | 'ethernet';

export type ConnectionState =
  | { status: 'Testing' }
  | { status: 'Connected'; detail: ConnectivityReport }
  | { status: 'Saving' }
  | { status: 'Saved'; detail: ConnectivityReport }
  | { status: 'Failed'; detail: string };

export interface WifiNetwork {
  id: string;
  bssids: string[];
  ssid: string;
  signal_strength: number;
  security: string;
}

export interface EthernetNetwork {
  interface: string;
  connected: boolean;
}

export interface SavedNetwork {
  id: string;
  transport: Transport;
  display_name: string;
  managed: boolean;
}

export interface IpAddress {
  address: string;
  prefix: number;
}

export interface IpConfig {
  method: 'auto' | 'manual';
  address_data?: IpAddress[];
  gateway?: string;
  dns?: string[];
}

export interface NetworkSettings {
  ssid?: string;
  password?: string;
  id?: string;
  eap_login?: string;
  eap_password?: string;
  ipv4?: IpConfig;
}

export interface Connectivity {
  is_connected: boolean;
  can_reach_host: boolean;
}

export interface AccessPointPreferences {
  persistent_enabled: boolean;
}

export interface AccessPointControllerStatus {
  ssid: string;
  boot_grace_seconds: number;
  is_enabled: boolean;
  persistent_enabled: boolean;
  automation_paused: boolean;
  last_reason: string;
}

export interface NetworkMacAddresses {
  wifi: string[];
  ethernet: string[];
}

export interface BrandingConfig {
  company_name: string;
  headline: string;
  support_text: string;
  success_message: string;
  logo_path: string;
  favicon_path: string;
  help_title: string;
  help_text: string;
  help_links: HelpLink[];
}

export interface HelpLink {
  label: string;
  url: string;
}

export interface ConnectivityReport {
  interface: string;
  address: string;
  gateway_reachable: boolean | null;
  internet_verified: boolean;
  endpoint_groups: EndpointGroupResult[];
}

export interface CurrentNetworkConfiguration {
  transport: Transport;
  ssid: string | null;
  ipv4_method: string;
  addresses: IpAddress[];
  gateway: string | null;
  dns: string[];
  mac_address: string | null;
}

export interface CurrentConnectionCheck {
  name: string;
  interface: string;
  report: ConnectivityReport | null;
  error: string | null;
  configuration: CurrentNetworkConfiguration | null;
  configuration_error: string | null;
}

export interface SavedNetworkDetails {
  network: SavedNetwork;
  settings: NetworkSettings;
  security: 'open' | 'personal' | 'enterprise';
  edit_unavailable: string | null;
}

export interface EndpointConfig {
  name: string;
  url: string;
  description: string;
}
export interface EndpointGroupConfig {
  name: string;
  required: boolean;
  endpoints: EndpointConfig[];
}
export interface EndpointResult extends EndpointConfig { reachable: boolean; }
export interface EndpointGroupResult extends EndpointGroupConfig { endpoints: EndpointResult[]; }
