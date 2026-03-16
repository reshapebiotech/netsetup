import API, { type types } from '$lib/api';

export const load = async ({ fetch }): Promise<types['Networks']> => {
  const { data: networks, error } = await API(fetch).GET('/wireless-networks');

  if (error) {
    return {
      networks: [],
    };
  }

  return networks ?? {};
};
