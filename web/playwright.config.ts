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
    headless: true,
    launchOptions: configuredChromiumPath ? { executablePath: configuredChromiumPath } : undefined
  }
});
