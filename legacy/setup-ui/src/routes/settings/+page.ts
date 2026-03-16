import API, { type types } from '$lib/api';

export const load = async ({ fetch }): Promise<types['AllSettings'] | undefined> => {
  const { data, error } = await API(fetch).GET('/network-settings');
  if (error) {
    console.error(error);
    return undefined;
  }

  return data;
};
