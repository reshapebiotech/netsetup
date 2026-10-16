import { readable } from 'svelte/store';
import type { BrandingConfig } from '#lib/network/types.js';

export const reshapeBranding: BrandingConfig = {
  product_name: 'Reshape Imaging Device',
  company_name: 'Reshape Biotech',
  headline: 'Network configuration',
  support_text: 'Connect this device to your network.',
  success_message: 'Your device is now online and ready to use.',
  logo_path: '/branding/logo.png',
  favicon_path: '/branding/favicon.ico',
  help_title: 'Need help?',
  help_text: 'Find troubleshooting and setup help below.',
  help_links: [{ label: 'Reshape setup article', url: 'https://support.reshapebiotech.com/en/articles/12785793-connecting-device-to-your-network' }]
};

export const branding = readable<BrandingConfig>(reshapeBranding);
