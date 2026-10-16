import { expect, test } from '@playwright/test';

const report = {
  interface: 'wlan0', address: '192.168.1.42', gateway_reachable: true,
  internet_verified: true, reshape_reachable: true
};

test.beforeEach(async ({ page }) => {
  await page.route('**/api/network/connectivity', route =>
    route.fulfill({ json: { is_connected: true, can_reach_host: true } }));
});

test('wifi testing reconnects, shows checks, and confirms save', async ({ page }) => {
  let saved = false;
  await page.route('**/api/network/saved', route =>
    route.fulfill({ json: saved ? [{
      id: 'managed', transport: 'wifi', display_name: 'reshape-managed-wifi', managed: true
    }] : [] }));
  await page.route('**/api/network/wifi/scan', route =>
    route.fulfill({ json: [{ ssid: 'LabWiFi', signal_strength: 80, security: 'WPA2' }] }));
  await page.route('**/api/network/wifi/test', route =>
    route.fulfill({ status: 202, json: { test_id: 'abc-123' } }));
  let hits = 0;
  await page.route('**/api/network/status/abc-123', route => {
    if (++hits === 1) return route.abort();
    return route.fulfill({ json: saved ? { status: 'Saved', detail: report } : { status: 'Connected', detail: report } });
  });
  await page.route('**/api/network/tests/abc-123/save', route => {
    saved = true;
    return route.fulfill({ status: 202 });
  });

  await page.goto('/setup/new/wifi');
  await page.getByRole('combobox', { name: 'SSID', exact: true }).selectOption('LabWiFi');
  await page.getByLabel('Password').fill('password');
  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByText('Reconnecting to your device…', { exact: true })).toBeVisible();
  await expect(page.getByText('Connected and ready', { exact: true })).toBeVisible();
  await expect(page.getByText('Internet access', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Save connection', exact: true }).click();
  await expect(page.getByText('Connection saved. Connected and ready.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Return home' }).click();
  await expect(page.getByText('reshape-managed-wifi', { exact: true })).toBeVisible();
});

test('local-only connection can be saved with an explicit warning', async ({ page }) => {
  await page.route('**/api/network/status/local', route => route.fulfill({
    json: { status: 'Connected', detail: { ...report, internet_verified: false, reshape_reachable: false } }
  }));
  await page.goto('/setup/new/wifi/result/local');
  await expect(page.getByText('Local network connected; internet could not be verified', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save anyway', exact: true })).toBeVisible();
  await expect(page.getByText('You can save this connection, but the device may be unable to communicate with Reshape.', { exact: false })).toBeVisible();
});

test('a failed save reports the restored connection rather than success', async ({ page }) => {
  let saving = false;
  await page.route('**/api/network/status/failure', route => route.fulfill({
    json: saving ? { status: 'Failed', detail: 'Verification failed; previous connection restored.' } : { status: 'Connected', detail: report }
  }));
  await page.route('**/api/network/tests/failure/save', route => {
    saving = true;
    return route.fulfill({ status: 202 });
  });
  await page.goto('/setup/new/wifi/save-handoff/failure');
  await expect(page.getByText('Verification failed; previous connection restored.', { exact: true })).toBeVisible();
  await expect(page.getByText('Connection saved. Connected and ready.')).not.toBeVisible();
});

test('a lost save response does not imply success', async ({ page }) => {
  await page.route('**/api/network/status/unknown', route => route.fulfill({
    json: { status: 'Connected', detail: report }
  }));
  await page.route('**/api/network/tests/unknown/save', route => route.abort());
  await page.goto('/setup/new/wifi/save-handoff/unknown');
  await expect(page.getByText('We could not confirm whether saving completed.', { exact: false })).toBeVisible();
  await expect(page.getByText('Connection saved. Connected and ready.')).not.toBeVisible();
});

test('open Wi-Fi submits without credentials and preserves limited connectivity after save', async ({ page }) => {
  const limited = { ...report, internet_verified: false, reshape_reachable: false };
  let saved = false;
  let submitted: Record<string, unknown> | null = null;
  await page.route('**/api/network/wifi/scan', route => route.fulfill({
    json: [{ ssid: 'OpenLab', signal_strength: 80, security: '--' }]
  }));
  await page.route('**/api/network/wifi/test', route => {
    submitted = route.request().postDataJSON();
    return route.fulfill({ status: 202, json: { test_id: 'open' } });
  });
  await page.route('**/api/network/status/open', route => route.fulfill({
    json: { status: saved ? 'Saved' : 'Connected', detail: limited }
  }));
  await page.route('**/api/network/tests/open/save', route => {
    saved = true;
    return route.fulfill({ status: 202 });
  });
  await page.goto('/setup/new/wifi');
  await expect(page.getByRole('combobox', { name: 'SSID', exact: true })).toHaveValue('OpenLab');
  await expect(page.getByLabel('Password')).toHaveCount(0);
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save anyway', exact: true })).toBeVisible();
  expect(submitted).not.toHaveProperty('password');
  await page.getByRole('button', { name: 'Save anyway', exact: true }).click();
  await expect(page.getByText('Connection saved. Local network connected; internet could not be verified.', { exact: true })).toBeVisible();
});

test('manual IPv4 preserves zero and rejects empty or fractional prefixes', async ({ page }) => {
  const submissions: Array<{ ipv4: { address_data: Array<{ prefix: number }> } }> = [];
  await page.route('**/api/network/ethernet/test', route => {
    submissions.push(route.request().postDataJSON());
    return route.fulfill({ status: 400, json: { error: 'Submission captured' } });
  });
  await page.goto('/setup/new/ethernet');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  await page.getByRole('combobox', { name: 'IPv4 method' }).selectOption('manual');
  await page.getByRole('textbox', { name: 'Address', exact: true }).fill('192.168.1.20');
  await page.getByRole('textbox', { name: 'Gateway', exact: true }).fill('192.168.1.1');
  const prefix = page.getByRole('spinbutton', { name: 'Prefix', exact: true });
  await prefix.fill('0');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page.getByText('Submission captured', { exact: true })).toBeVisible();
  expect(submissions[0].ipv4.address_data[0].prefix).toBe(0);
  for (const invalid of ['', '24.5']) {
    await prefix.fill(invalid);
    await page.getByRole('button', { name: 'Test connection', exact: true }).click();
    await expect(page.getByText('IPv4 prefix must be an integer between 0 and 32', { exact: true })).toBeVisible();
    expect(submissions).toHaveLength(1);
  }
});
