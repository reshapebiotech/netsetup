import { defineConfig } from '@playwright/test';
import { existsSync } from 'node:fs';

const macChromiumPath = '/Applications/Chromium.app/Contents/MacOS/Chromium';
const configuredChromiumPath =
  process.env.PLAYWRIGHT_CHROMIUM_PATH ??
  (existsSync(macChromiumPath) ? macChromiumPath : undefined);

export default defineConfig({
  testDir: 'tests/e2e',
  webServer: {
    command: 'bun run dev',
    url: process.env.E2E_BASE_URL ?? 'http://localhost:5173',
    reuseExistingServer: true,
    timeout: 60_000
  },
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:5173',
    trace: 'retain-on-failure',
    headless: true
  },
  projects: [
    { name: 'chromium', use: {
      browserName: 'chromium',
      launchOptions: configuredChromiumPath ? { executablePath: configuredChromiumPath } : undefined
    } },
    { name: 'firefox', grep: /spinner rotates normally|SSID selection/, use: { browserName: 'firefox' } },
    { name: 'webkit', grep: /handoff page|DNS help|password reveal|spinner rotates normally|SSID selection/, use: {
      browserName: 'webkit', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true
    } }
  ]
});
