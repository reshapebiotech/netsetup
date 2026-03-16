import { writable } from 'svelte/store';
import type { types } from './api';

export const theme = writable<'light' | 'dark'>('light');

export const robotMetadata = writable<{ name: string | undefined }>({
  name: undefined,
});

export const serverStateUrl = writable<types['State-Input']['state']>('/');

export const networks = writable<types['Networks'] | undefined>(undefined);
export const hasNoEthernet = writable<boolean>(false);
export const hasEapEthernet = writable<boolean>(false);
export const hasInternet = writable<boolean>(false);

export const isConnectedToRobot = writable<boolean>(true);

export const networkConfig = writable<{
  ssid: string;
  password: string;
  eap_login: string;
  eap_password: string;
  ipv4_method: 'auto' | 'manual';
  ipv4_address: string;
  ipv4_prefix: string;
  ipv4_gateway: string;
  dns: string[];
}>({
  ssid: '',
  password: '',
  eap_login: '',
  eap_password: '',
  ipv4_method: 'auto' as 'auto' | 'manual',
  ipv4_address: '',
  ipv4_prefix: '24',
  ipv4_gateway: '',
  dns: [],
});

export const dnsMethod = writable<'auto' | 'manual'>('auto');

export const isLoading = writable<boolean>(false);

export const apiURL = writable<string>('');
