import { expect, test } from '@playwright/test';

const groups = [
  { name: 'Core services', required: true, endpoints: [
    { name: 'Device API', url: 'https://api.example.com', description: 'Device communication' },
    { name: 'Plate images', url: 'https://images.example.com', description: 'Image storage' }
  ] },
  { name: 'Device management', required: false, endpoints: [
    { name: 'Fleet', url: 'https://fleet.example.com', description: 'Remote management' },
    { name: 'Updates', url: 'https://updates.example.com', description: 'Firmware downloads' }
  ] }
];
const report = {
  interface: 'wlan0', address: '192.168.1.42', gateway_reachable: true,
  internet_verified: true,
  endpoint_groups: groups.map(group => ({ ...group, endpoints: group.endpoints.map(endpoint => ({ ...endpoint, reachable: true })) }))
};
const unavailableGroups = report.endpoint_groups.map(group => ({ ...group,
  endpoints: group.endpoints.map(endpoint => ({ ...endpoint, reachable: false }))
}));

test.beforeEach(async ({ page }) => {
  await page.route('**/api/network/connectivity/groups', route => route.fulfill({ json: groups }));
  await page.route('**/api/device', route => route.fulfill({ json: { hostname: 'RS-12345' } }));
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
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByText('Reconnecting to your device…', { exact: true })).toBeVisible();
  await expect(page.getByText('Full service connectivity', { exact: true })).toBeVisible();
  await expect(page.getByText('Internet access', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Save connection', exact: true }).click();
  await expect(page.getByText('Connection saved. Full service connectivity.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Return home' }).click();
  await expect(page.getByText('reshape-managed-wifi', { exact: true })).toBeVisible();
});

test('local-only connection can be saved with an explicit warning', async ({ page }) => {
  await page.route('**/api/network/status/local', route => route.fulfill({
    json: { status: 'Connected', detail: { ...report, internet_verified: false, endpoint_groups: unavailableGroups } }
  }));
  await page.goto('/setup/new/wifi/result/local');
  await expect(page.getByText('No service connectivity', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save anyway', exact: true })).toBeVisible();
  await expect(page.getByText('Some configured services could not be reached. You can still save this connection if local access is sufficient.', { exact: false })).toBeVisible();
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
  await expect(page.getByText('Connection saved. Full service connectivity.')).not.toBeVisible();
});

test('a lost save response does not imply success', async ({ page }) => {
  await page.route('**/api/network/status/unknown', route => route.fulfill({
    json: { status: 'Connected', detail: report }
  }));
  await page.route('**/api/network/tests/unknown/save', route => route.abort());
  await page.goto('/setup/new/wifi/save-handoff/unknown');
  await expect(page.getByText('We could not confirm whether saving completed.', { exact: false })).toBeVisible();
  await expect(page.getByText('Connection saved. Full service connectivity.')).not.toBeVisible();
});

test('open Wi-Fi submits without credentials and preserves limited connectivity after save', async ({ page }) => {
  const limited = { ...report, internet_verified: false, endpoint_groups: unavailableGroups };
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
  await expect(page.getByLabel('Password', { exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save anyway', exact: true })).toBeVisible();
  expect(submitted).not.toHaveProperty('password');
  await page.getByRole('button', { name: 'Save anyway', exact: true }).click();
  await expect(page.getByText('Connection saved. No service connectivity.', { exact: true })).toBeVisible();
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

test('password reveal preserves the field and allows focus to leave', async ({ page }) => {
  await page.clock.install();
  let connectivityPolls = 0;
  await page.route('**/api/network/connectivity', route => {
    connectivityPolls++;
    return route.fulfill({ json: { is_connected: true, can_reach_host: true } });
  });
  await page.route('**/api/network/wifi/scan', route => route.fulfill({
    json: [{ ssid: 'LabWiFi', signal_strength: 80, security: 'WPA2' }]
  }));
  await page.goto('/setup/new/wifi');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  const password = page.getByLabel('Password', { exact: true });
  await password.fill('test-password');
  await page.getByRole('button', { name: 'Show password', exact: true }).click();
  await expect(password).toHaveAttribute('type', 'text');
  await expect(password).toHaveValue('test-password');
  await page.getByRole('button', { name: 'Hide password', exact: true }).click();
  await expect(password).toHaveAttribute('type', 'password');
  await expect(password).toHaveValue('test-password');
  const dns = page.getByLabel('Custom DNS servers', { exact: false });
  await dns.fill('1.1.1.1');
  await expect(dns).toBeFocused();
  await page.getByRole('heading', { name: 'IPv4 and DNS', exact: true }).click();
  await page.clock.runFor(5_000);
  await expect.poll(() => connectivityPolls).toBeGreaterThan(1);
  await expect(password).not.toBeFocused();
  await password.focus();
  await password.press('Tab');
  await expect(page.getByRole('button', { name: 'Show password', exact: true })).toBeFocused();
});

for (const [seconds, expected] of [[90, '90 seconds'], [120, '2 minutes']] as const) {
  test(`help shows the device startup window: ${expected}`, async ({ page }) => {
    await page.route('**/api/network/saved', route => route.fulfill({ json: [] }));
    await page.route('**/api/network/access_point/controller', route => route.fulfill({
      json: { ssid: 'testpi', boot_grace_seconds: seconds, is_enabled: false, persistent_enabled: false,
        automation_paused: false, last_reason: '' }
    }));
    await page.goto('/');
    await page.getByRole('link', { name: 'Help', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Network setup help', exact: true })).toBeVisible();
    await expect(page.getByText('testpi', { exact: true })).toBeVisible();
    await expect(page.getByText('The device creates its own Wi-Fi network', { exact: false })).toBeVisible();
    await page.locator('summary').filter({ hasText: 'I can’t find setup Wi-Fi' }).click();
    await expect(page.getByText(`saved networks have ${expected}`, { exact: false })).toBeVisible();
    await expect(page.getByText('Losing internet later does not automatically open setup Wi-Fi.', { exact: false })).toBeVisible();
  });
}

test('help remains readable when device settings cannot be loaded', async ({ page }) => {
  await page.route('**/api/network/access_point/controller', route => route.abort());
  await page.goto('/help');
  await expect(page.locator('summary').filter({ hasText: 'Setup Wi-Fi disappeared' })).toBeVisible();
  await page.locator('summary').filter({ hasText: 'I can’t find setup Wi-Fi' }).click();
  await expect(page.getByText('it gives saved networks a startup window', { exact: false })).toBeVisible();
  await expect(page.getByText('90 seconds', { exact: false })).toHaveCount(0);
});


test('help examples are read-only and never start network operations', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const mutations: string[] = [];
  await page.route('**/api/network/**', route => {
    if (route.request().method() !== 'GET') mutations.push(route.request().url());
    return route.fulfill({ json: route.request().url().endsWith('/connectivity/groups') ? groups : {} });
  });
  await page.goto('/help');
  await expect(page.getByText('No internet', { exact: true })).toBeVisible();
  await page.locator('summary').filter({ hasText: 'What does my test result mean?' }).click();
  const preview = page.locator('figure');
  await expect(preview.getByText('No service connectivity', { exact: true })).toBeVisible();
  await page.getByLabel('Choose the result you saw').selectOption('0');
  await expect(preview.getByText('Full service connectivity', { exact: true })).toBeVisible();
  await page.getByLabel('Choose the result you saw').selectOption('2');
  await expect(preview.getByText('Required services available; other services limited', { exact: true })).toBeVisible();
  await expect(preview.locator('button, input, select, textarea, a')).toHaveCount(0);
  expect(mutations).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});


test('header navigation and current connectivity checks do not change networks', async ({ page }) => {
  const mutations: string[] = [];
  let checks = 0;
  await page.route('**/api/network/**', route => {
    if (route.request().method() !== 'GET') mutations.push(route.request().url());
    if (route.request().url().endsWith('/connectivity/check')) {
      checks++;
      return route.fulfill({ json: checks === 1 ? [{ name: 'Lab Wi-Fi', interface: 'wlan0', report, error: null }] : [] });
    }
    if (route.request().url().endsWith('/connectivity')) return route.fulfill({ json: { is_connected: true, can_reach_host: true } });
    return route.fulfill({ json: [] });
  });
  await page.goto('/');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  await expect(page.locator('a[href^="https://support."]')).toHaveCount(0);
  await expect(page.locator('header').first().getByText('RS-12345', { exact: true })).toBeVisible();
  const nav = page.getByRole('navigation', { name: 'Main navigation' });
  await expect(nav.getByRole('link', { name: 'Settings', exact: true })).toBeVisible();
  await nav.getByRole('link', { name: 'Connectivity', exact: true }).click();
  await expect(nav.getByRole('link', { name: 'Connectivity', exact: true })).toHaveAttribute('aria-current', 'page');
  expect(checks).toBe(0);
  await page.getByRole('button', { name: 'Check connectivity' }).click();
  await expect(page.getByText('Full service connectivity', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Check connectivity' }).click();
  await expect(page.getByText('No active Wi-Fi or Ethernet connection to check.', { exact: false })).toBeVisible();
  await expect(page.getByText('Full service connectivity', { exact: true })).toHaveCount(0);
  expect(mutations).toEqual([]);
});


test('saved networks can be inspected and edited without sending the stored password', async ({ page }) => {
  const network = { id: 'saved-wifi', transport: 'wifi', display_name: 'Lab Wi-Fi', managed: true };
  const settings = { ssid: 'Lab Wi-Fi', ipv4: { method: 'manual', address_data: [{ address: '192.168.1.20', prefix: 24 }], gateway: '192.168.1.1', dns: ['9.9.9.9'] } };
  await page.route('**/api/network/saved', route => route.fulfill({ json: [network] }));
  await page.route('**/api/network/saved/saved-wifi', route => route.fulfill({ json: { network, settings, security: 'personal', edit_unavailable: null } }));
  await page.route('**/api/network/wifi/scan', route => route.fulfill({ json: [] }));
  let submitted: Record<string, unknown> | null = null;
  await page.route('**/api/network/wifi/test', route => {
    submitted = route.request().postDataJSON();
    return route.fulfill({ status: 400, json: { error: 'Submission captured' } });
  });
  await page.goto('/');
  await page.getByRole('link', { name: 'Lab Wi-Fi', exact: false }).click();
  await expect(page.getByText('192.168.1.20/24', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Edit connection' }).click();
  await expect(page.getByRole('textbox', { name: 'SSID', exact: true })).toHaveValue('Lab Wi-Fi');
  await expect(page.getByRole('textbox', { name: 'Address', exact: true })).toHaveValue('192.168.1.20');
  await expect(page.getByLabel('Keep saved password')).toBeChecked();
  await page.getByLabel('Custom DNS servers', { exact: false }).fill('1.1.1.1');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page.getByText('Submission captured')).toBeVisible();
  expect(submitted).toMatchObject({ id: 'saved-wifi', ssid: 'Lab Wi-Fi', ipv4: { dns: ['1.1.1.1'] } });
  expect(submitted).not.toHaveProperty('password');
  await page.getByLabel('Keep saved password').uncheck();
  await page.getByLabel('Password', { exact: true }).fill('replacement-password');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect.poll(() => submitted).toMatchObject({ password: 'replacement-password' });
  expect(submitted).not.toHaveProperty('id');
});


test('partial service results identify optional failures without blocking normal save', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 740 });
  const partial = { ...report, endpoint_groups: report.endpoint_groups.map(group => ({
    ...group, endpoints: group.endpoints.map(endpoint => ({ ...endpoint, reachable: endpoint.name !== 'Updates' }))
  })) };
  await page.route('**/api/network/status/partial', route => route.fulfill({ json: { status: 'Connected', detail: partial } }));
  await page.goto('/setup/new/wifi/result/partial');
  await expect(page.getByText('Required services available; other services limited', { exact: true })).toBeVisible();
  const core = page.getByRole('region', { name: 'Core services', exact: true });
  const management = page.getByRole('region', { name: 'Device management', exact: true });
  await expect(core.getByText('Full connectivity', { exact: true })).toBeVisible();
  await expect(management.getByText('Partial connectivity', { exact: true })).toBeVisible();
  await expect(management.getByText('Could not reach', { exact: true })).toBeVisible();
  await expect(page.getByText('https://updates.example.com', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save connection', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
