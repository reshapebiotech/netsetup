import { goto } from '$app/navigation';
import { get } from 'svelte/store';
import API, { type types } from './api';
import { networkConfig, isLoading, robotMetadata } from './stores';

export type trustedPathnames =
  | '/'
  | '/setup/one'
  | '/setup/two'
  | '/setup/three'
  | '/setup/ethernet/one'
  | '/setup/ethernet/two'
  | '/setup/ethernet/three'
  | '/setup/wifi/one'
  | '/setup/wifi/connecting'
  | '/settings';

export const getNetworkSettings = async () => {
  isLoading.set(true);
  const { data, error } = await API().GET('/network-settings');
  isLoading.set(false);
  if (error) {
    console.error(error);
    return undefined;
  }
  return data.settings;
};

export const resetSettings = async () => {
  const confirmed = confirm('Are you sure? This will reset all network settings.');

  if (!confirmed) {
    return;
  }

  isLoading.set(true);
  const hostName = get(robotMetadata).name?.toUpperCase();
  const settings = await getNetworkSettings();
  isLoading.set(false);
  if (!settings) {
    return;
  }

  const filtered = settings?.filter((setting) => setting.ssid !== hostName) ?? [];
  isLoading.set(true);
  const promises = filtered.map((setting) => {
    return API().DELETE('/network-settings/{id}', {
      params: { path: { id: setting.id ?? '' } },
    });
  });

  const results = await Promise.all(promises);
  isLoading.set(false);
  return results;
};

export const connectWired = async (input: types['NetworkSettings-Input']) => {
  isLoading.set(true);
  const { data, error } = await API().POST('/ethernet-settings', {
    body: input || {},
  });
  isLoading.set(false);

  if (error) {
    console.error(error);
  }

  return { data, error };
};

export const resetNetworkConfig = () => {
  networkConfig.update(() => {
    return {
      ssid: '',
      password: '',
      eap_login: '',
      eap_password: '',
      ipv4_method: 'auto',
      ipv4_address: '',
      ipv4_prefix: '24',
      ipv4_gateway: '',
      dns: [],
    };
  });
};
