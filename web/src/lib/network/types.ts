export type Transport = 'wifi' | 'ethernet';

export type ConnectionState =
  | { status: 'Testing' }
  | { status: 'Connected'; detail: ConnectivityReport }
  | { status: 'Saving' }
  | { status: 'Saved'; detail: ConnectivityReport }
  | { status: 'Failed'; detail: string };

export interface WifiNetwork {
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
  product_name: string;
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
  reshape_reachable: boolean;
}
