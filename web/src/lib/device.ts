import { createContext } from 'svelte';

export const [getDevice, setDevice] = createContext<{ hostname: string | null }>();
