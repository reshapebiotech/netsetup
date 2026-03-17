import { expect, test } from '@playwright/test';

test('wifi test flow shows handoff copy', async ({ page }) => {
  await page.route('**/api/network/wifi/scan', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([{ ssid: 'LabWiFi', signal_strength: 80, security: 'WPA2' }])
    });
  });

  await page.route('**/api/network/wifi/test', async (route) => {
    await route.fulfill({
      status: 202,
      contentType: 'application/json',
      body: JSON.stringify({ test_id: 'abc-123' })
    });
  });

  let statusHits = 0;
  await page.route('**/api/network/status/abc-123', async (route) => {
    statusHits += 1;
    if (statusHits === 1) {
      await route.abort();
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ status: 'Connected' })
    });
  });

  await page.route('**/api/network/tests/abc-123/save', async (route) => {
    await route.fulfill({ status: 202, body: '' });
  });

  await page.goto('/setup/new/wifi');
  await page.getByRole('combobox', { name: 'SSID', exact: true }).selectOption('LabWiFi');
  await page.getByLabel('Password').fill('password');
  const startReq = page.waitForRequest('**/api/network/wifi/test');
  await page.getByRole('button', { name: 'Test connection' }).click({ force: true });
  await startReq;
  await page.waitForURL('**/setup/new/wifi/testing/abc-123');

  await expect(page.getByText('Hold tight while we switch networks')).toBeVisible();
  await expect(page.getByText('Reconnecting to setup API…', { exact: true })).toBeVisible();

  await expect(page.getByText('test complete')).toBeVisible();
  await page.getByRole('button', { name: 'Save connection' }).click();

  await expect(page.getByText('Applying your saved network')).toBeVisible();
  await expect(page.getByText('This is expected behavior: during save')).toBeVisible();
});
