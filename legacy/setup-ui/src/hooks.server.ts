import type { Handle } from '@sveltejs/kit';
import { sequence } from '@sveltejs/kit/hooks';

const headerHandler: Handle = async ({ event, resolve }) => {
  const response = await resolve(event, {
    filterSerializedResponseHeaders: (name: string) => name === 'content-length',
  });
  return response;
};

export const handle = sequence(headerHandler);
