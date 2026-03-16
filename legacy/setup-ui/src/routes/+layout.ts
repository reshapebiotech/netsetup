import { robotMetadata, apiURL } from '$lib/stores';
import type { LayoutLoad } from './$types';
import { env } from '$env/dynamic/public';

export const load: LayoutLoad = async ({ url }) => {
  const hostname = env.PUBLIC_DEVICE_NAME?.toUpperCase();

  const hostInformation = {
    name: hostname ?? 'DEVICE',
  };

  apiURL.set(`${env.PUBLIC_API_PREFIX ?? 'http://'}${url.hostname}:${env.PUBLIC_API_PORT ?? 9000}`);
  robotMetadata.set(hostInformation);

  return {
    connectivity: { is_connected: false },
    url,
  };
};
