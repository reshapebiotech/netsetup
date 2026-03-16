import createClient from 'openapi-fetch';
import type { paths, components } from './schemas/schema';
import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';
import { apiURL } from './stores';
import { get } from 'svelte/store';

let baseURL = get(apiURL) ?? env.PUBLIC_API_ENDPOINT ?? `http://${env.PUBLIC_DEVICE_NAME}:9000`;

apiURL.subscribe((url) => {
  baseURL = url;
});

export const API = (fetchOverride?: typeof fetch) => {
  if (!fetchOverride && !browser) {
    console.warn(
      `
No fetch implementation was present when calling the API. This will fall back to the (possibly undefined) global fetch.
If you are fetching data on the server, make sure to pass the fetch implementation as an argument to the API function.`
    );
    console.trace();
  }

  return createClient<paths>({
    baseUrl: baseURL,
    fetch: fetchOverride ?? fetch,
    headers: { 'Content-Type': 'application/json' },
  });
};

export type types = components['schemas'];

export default API;
