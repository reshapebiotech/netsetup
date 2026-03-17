import { writable } from 'svelte/store';
import type { BrandingConfig } from '$lib/network/types';
import { networkApi } from '$lib/network/api';

export const defaultBranding: BrandingConfig = {
  product_name: 'Network Setup',
  company_name: 'Reshape Biotech',
  headline: 'Network configuration',
  support_text: 'Connect this device to your network.',
  success_message: 'Your device is now online and ready to use.',
  logo_path: '/branding/logo.png',
  background_path: '',
  favicon_path: '/branding/favicon.ico',
  help_title: 'Need help?',
  help_text: 'Use the resources below for setup guidance.',
  help_links: []
};

export const branding = writable<BrandingConfig>(defaultBranding);

export async function loadBranding(): Promise<void> {
  try {
    const value = await networkApi.getBranding();
    branding.set(value);
  } catch {
    branding.set(defaultBranding);
  }
}
