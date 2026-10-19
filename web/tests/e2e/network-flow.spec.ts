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
const configuration = {
  transport: 'wifi', ssid: 'Lab Wi-Fi', ipv4_method: 'auto',
  addresses: [{ address: '192.168.11.0', prefix: 23 }, { address: '10.0.0.5', prefix: 32 }],
  gateway: '192.168.10.1', dns: ['192.168.10.1', '1.1.1.1'], mac_address: '02:00:00:00:00:01'
};
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
    route.fulfill({ json: [{ id: 'lab-wifi', bssids: ['02:00:00:00:01:01'], ssid: 'LabWiFi', signal_strength: 80, security: 'WPA2' }] }));
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
  await page.getByRole('combobox', { name: 'SSID', exact: true }).selectOption('lab-wifi');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByText('Waiting to reconnect to your device…', { exact: true })).toBeVisible();
  await expect(page.getByText('Full service connectivity', { exact: true })).toBeVisible();
  await expect(page.getByText('Internet access', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Save connection', exact: true }).click();
  await expect(page.getByText('Connection saved. Full service connectivity.', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Open netsetup', exact: true })).toHaveAttribute('href', 'http://192.168.1.42:3000/setup/new/wifi/save-handoff/abc-123');
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
  await expect(page.getByRole('link', { name: 'Open netsetup', exact: true })).toHaveAttribute('href', 'http://192.168.1.42:3000/setup/new/wifi/save-handoff/unknown');
  await expect(page.getByText('Saving has not been confirmed.', { exact: false })).toBeVisible();
});

test('open Wi-Fi submits without credentials and preserves limited connectivity after save', async ({ page }) => {
  const limited = { ...report, internet_verified: false, endpoint_groups: unavailableGroups };
  let saved = false;
  let submitted: Record<string, unknown> | null = null;
  await page.route('**/api/network/wifi/scan', route => route.fulfill({
    json: [{ id: 'open-lab', bssids: ['02:00:00:00:02:01'], ssid: 'OpenLab', signal_strength: 80, security: '--' }]
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
  await page.getByRole('combobox', { name: 'SSID', exact: true }).selectOption('open-lab');
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

test('invalid manual IPv4 combinations are blocked before a network test starts', async ({ page }) => {
  let submissions = 0;
  await page.route('**/api/network/ethernet/test', route => {
    submissions++;
    return route.fulfill({ status: 400, json: { error: 'Submission captured' } });
  });
  await page.goto('/setup/new/ethernet');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  await page.getByRole('combobox', { name: 'IPv4 method' }).selectOption('manual');
  const address = page.getByRole('textbox', { name: 'Address', exact: true });
  const gateway = page.getByRole('textbox', { name: 'Gateway', exact: true });
  const prefix = page.getByRole('spinbutton', { name: 'Prefix', exact: true });
  await prefix.fill('24');
  for (const [ip, gw, error] of [
    ['192.168.1.20', '192.168.2.1', 'Gateway must be in the same subnet as a manual IPv4 address'],
    ['192.168.1.20', '192.168.1.20', 'Gateway must differ from the device IP address'],
    ['192.168.1.0', '192.168.1.1', "Manual IPv4 address must not be the subnet's network or broadcast address"],
    ['192.168.1.20', '192.168.1.255', "Gateway must not be the subnet's network or broadcast address"]
  ]) {
    await address.fill(ip);
    await gateway.fill(gw);
    await page.getByRole('button', { name: 'Test connection', exact: true }).click();
    await expect(page.getByText(error, { exact: true })).toBeVisible();
    expect(submissions).toBe(0);
  }
  await prefix.fill('31');
  await address.fill('10.0.0.4');
  await gateway.fill('10.0.0.5');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page.getByText('Submission captured', { exact: true })).toBeVisible();
  expect(submissions).toBe(1);
});

test('password reveal preserves the field and allows focus to leave', async ({ page, browserName }) => {
  await page.clock.install();
  let connectivityPolls = 0;
  await page.route('**/api/network/connectivity', route => {
    connectivityPolls++;
    return route.fulfill({ json: { is_connected: true, can_reach_host: true } });
  });
  await page.route('**/api/network/wifi/scan', route => route.fulfill({
    json: [{ id: 'lab-wifi', bssids: ['02:00:00:00:01:01'], ssid: 'LabWiFi', signal_strength: 80, security: 'WPA2' }]
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
  expect(connectivityPolls).toBe(1);
  await expect(password).not.toBeFocused();
  await password.focus();
  await password.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
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
    await expect(page.getByRole('heading', { name: 'netsetup help', exact: true })).toBeVisible();
    await expect(page.getByText('testpi', { exact: true })).toBeVisible();
    await expect(page.getByText('To open netsetup, connect to testpi in your phone’s Wi-Fi settings.', { exact: true })).toBeVisible();
    await page.locator('summary').filter({ hasText: 'I can’t open netsetup' }).click();
    await expect(page.getByText(`saved networks have ${expected}`, { exact: false })).toBeVisible();
    await expect(page.getByText('the device’s network also opens if all saved Wi-Fi and Ethernet profiles have been removed', { exact: false })).toBeVisible();
    await expect(page.getByText('Losing internet later does not automatically open the device’s network while saved networks remain.', { exact: false })).toBeVisible();
  });
}

test('help remains readable when device settings cannot be loaded', async ({ page }) => {
  await page.route('**/api/network/access_point/controller', route => route.abort());
  await page.goto('/help');
  await expect(page.locator('summary').filter({ hasText: 'Connection lost during a test' })).toBeVisible();
  await page.locator('summary').filter({ hasText: 'I can’t open netsetup' }).click();
  await expect(page.getByText('it gives saved networks a startup window', { exact: false })).toBeVisible();
  await expect(page.getByText('90 seconds', { exact: false })).toHaveCount(0);
});


test('help provides partial connectivity guidance without changing networks', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const mutations: string[] = [];
  await page.route('**/api/network/**', route => {
    if (route.request().method() !== 'GET') mutations.push(route.request().url());
    return route.fulfill({ json: route.request().url().endsWith('/connectivity/groups') ? groups : {} });
  });
  await page.goto('/help');
  await expect(page.getByText('No internet', { exact: true })).toBeVisible();
  await page.locator('summary').filter({ hasText: 'Some services couldn’t be reached' }).click();
  await expect(page.getByText('Take a screenshot of the results and share it with your IT department or Reshape support.', { exact: false })).toBeVisible();
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
      return route.fulfill({ json: checks === 1 ? [{ name: 'Lab Wi-Fi', interface: 'wlan0', report, error: null, configuration, configuration_error: null }] : [] });
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
  const details = page.getByRole('region', { name: 'Current network configuration' });
  await expect(details.getByText('192.168.11.0/23', { exact: true })).toBeVisible();
  await expect(details.getByText('10.0.0.5/32', { exact: true })).toBeVisible();
  await expect(details.getByText('Automatic (DHCP)', { exact: true })).toBeVisible();
  await expect(details.getByText('02:00:00:00:00:01', { exact: true })).toBeVisible();
  await expect(page.locator('time')).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T/);
  const serviceDetails = page.locator('details').filter({ has: page.getByText('Device API', { exact: true }) });
  await expect(serviceDetails.getByText('https://api.example.com', { exact: true })).not.toBeVisible();
  await serviceDetails.locator('summary').click();
  await expect(serviceDetails.getByText('https://api.example.com', { exact: true })).toBeVisible();
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


test('network details remain readable on a phone when connectivity probes fail', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route('**/api/network/connectivity/check', route => route.fulfill({ json: [{
    name: 'Lab Wi-Fi', interface: 'wlan0', report: null, error: 'No usable IPv4 address.',
    configuration, configuration_error: null
  }] }));
  await page.goto('/connectivity');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Check connectivity', exact: true }).click();
  await expect(page.getByText('No usable IPv4 address.', { exact: true })).toBeVisible();
  const details = page.getByRole('region', { name: 'Current network configuration' });
  await expect(details.getByText('192.168.11.0/23', { exact: true })).toBeVisible();
  await expect(details.getByText('192.168.10.1, 1.1.1.1', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Lab Wi-Fi', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

for (const reducedMotion of ['reduce', 'no-preference'] as const) {
  test(`spinner rotates normally with motion preference: ${reducedMotion}`, async ({ page }) => {
    await page.emulateMedia({ reducedMotion });
    let finish!: () => void;
    const gate = new Promise<void>(resolve => { finish = resolve; });
    await page.route('**/api/network/connectivity/check', async route => {
      await gate;
      await route.fulfill({ json: [] });
    });
    try {
      await page.goto('/connectivity');
      await expect(page.getByText('Online', { exact: true })).toBeVisible();
      await page.getByRole('button', { name: 'Check connectivity', exact: true }).click();
      const spinner = page.getByRole('button', { name: 'Checking…', exact: true }).locator('.animate-spin');
      await expect(spinner).toHaveCSS('animation-name', 'netsetup-spin');
      await expect(spinner).toHaveCSS('animation-duration', '1s');
      await expect(spinner).toHaveCSS('animation-iteration-count', 'infinite');
      const initialTransform = await spinner.evaluate(element => getComputedStyle(element).transform);
      await expect.poll(() => spinner.evaluate(element => getComputedStyle(element).transform))
        .not.toBe(initialTransform);
    } finally {
      finish();
    }
    await expect(page.getByRole('button', { name: 'Check connectivity', exact: true })).toBeEnabled();
  });
}


test('handoff page loads before the Wi-Fi test can disconnect the browser', async ({ page }) => {
  await page.route('**/api/network/access_point/controller', route => route.fulfill({ json: { ssid: 'Setup-Lab-0440' } }));
  let started = false;
  const lateScripts: string[] = [];
  await page.route('**/*', route => {
    if (started && route.request().resourceType() === 'script') {
      lateScripts.push(route.request().url());
      return route.abort();
    }
    return route.fallback();
  });
  await page.route('**/api/network/wifi/scan', route => route.fulfill({ json: [{ id: 'lab-wifi', bssids: ['02:00:00:00:01:01'], ssid: 'LabWiFi', signal_strength: 80, security: 'WPA2' }] }));
  await page.route('**/api/network/wifi/test', route => {
    started = true;
    return route.fulfill({ status: 202, json: { test_id: 'offline' } });
  });
  await page.route('**/api/network/status/offline', route => route.abort());
  await page.goto('/setup/new/wifi');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  await page.getByRole('combobox', { name: 'SSID', exact: true }).selectOption('lab-wifi');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page).toHaveURL(/\/testing\/offline$/);
  await expect(page.getByRole('heading', { name: 'Stay connected to netsetup', exact: true })).toBeVisible();
  await expect(page.getByText('Waiting to reconnect to your device…', { exact: true })).toBeVisible();
  await expect(page.getByText('Setup-Lab-0440', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('If asked, choose to stay connected without internet.', { exact: true })).toBeVisible();
  await expect(page.getByText('Wait for the test result before switching to your usual Wi-Fi.', { exact: true })).toBeVisible();
  expect(lateScripts).toEqual([]);
});

test('DNS help does not activate its input or restart idle form focus', async ({ page }) => {
  await page.clock.install();
  let connectivityPolls = 0;
  await page.route('**/api/network/connectivity', route => {
    connectivityPolls++;
    return route.fulfill({ json: { is_connected: true, can_reach_host: true } });
  });
  await page.goto('/setup/new/ethernet');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  const dns = page.getByLabel('Custom DNS servers', { exact: false });
  await dns.fill('1.1.1.1');
  await dns.blur();
  await page.getByText('Leave empty to use DNS servers provided by DHCP.', { exact: false }).click();
  await page.clock.runFor(15_000);
  await expect(dns).not.toBeFocused();
  expect(connectivityPolls).toBe(1);
  await page.getByRole('navigation', { name: 'Main navigation' }).getByRole('link', { name: 'Connectivity', exact: true }).click();
  await page.clock.runFor(5_000);
  await expect.poll(() => connectivityPolls).toBeGreaterThan(1);
});


test('opening the device address confirms a saved connection without saving again', async ({ page }) => {
  let mutations = 0;
  await page.route('**/api/network/tests/confirmed/save', route => {
    mutations++;
    return route.abort();
  });
  await page.route('**/api/network/status/confirmed', route => route.fulfill({ json: { status: 'Saved', detail: report } }));
  await page.goto('/setup/new/wifi/save-handoff/confirmed');
  await expect(page.getByText('Connection saved. Full service connectivity.', { exact: true })).toBeVisible();
  expect(mutations).toBe(0);
});


test('SSID selection remains explicit and matches submissions across rescans', async ({ page }) => {
  const company = ' Company Wi-Fi ';
  let scans = 0;
  const submissions: Array<{ ssid: string }> = [];
  await page.route('**/api/network/wifi/scan', route => {
    scans++;
    const networks = scans === 1
      ? [{ id: 'other-network', bssids: ['02:00:00:00:03:01'], ssid: 'Other network', signal_strength: 99, security: 'WPA2' }, { id: 'company-wifi', bssids: ['02:00:00:00:04:01'], ssid: company, signal_strength: 70, security: 'WPA2' }]
      : scans === 2
        ? [{ id: 'company-wifi', bssids: ['02:00:00:00:04:01'], ssid: company, signal_strength: 95, security: 'WPA2' }, { id: 'other-network', bssids: ['02:00:00:00:03:01'], ssid: 'Other network', signal_strength: 60, security: 'WPA2' }]
        : [{ id: 'other-network', bssids: ['02:00:00:00:03:01'], ssid: 'Other network', signal_strength: 99, security: 'WPA2' }];
    return route.fulfill({ json: networks });
  });
  await page.route('**/api/network/wifi/test', route => {
    submissions.push(route.request().postDataJSON());
    return route.fulfill({ status: 400, json: { error: 'Submission captured' } });
  });
  await page.goto('/setup/new/wifi');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  const selector = page.getByRole('combobox', { name: 'SSID', exact: true });
  await expect(selector).toBeEnabled();
  await expect(selector).toHaveValue('');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect(page.getByText('SSID is required for Wi-Fi', { exact: true })).toBeVisible();
  expect(submissions).toHaveLength(0);
  await selector.selectOption('company-wifi');
  for (const expectedScans of [2, 3]) {
    await page.getByRole('button', { name: 'Rescan', exact: true }).click();
    await expect.poll(() => scans).toBe(expectedScans);
    await expect(selector).toBeEnabled();
    await expect(selector).toHaveValue('company-wifi');
    await expect(selector.locator('option:checked')).toContainText('Company Wi-Fi');
    if (expectedScans === 3) await expect(selector.locator('option:checked')).toContainText('not seen in latest scan');
    await page.getByRole('button', { name: 'Test connection', exact: true }).click();
    await expect.poll(() => submissions.length).toBe(expectedScans - 1);
    expect(submissions.at(-1)?.ssid).toBe(company);
  }
});


test('SSID selection distinguishes security groups without pinning a radio', async ({ page }) => {
  const networks = [
    { id: 'shared-open', ssid: 'Shared Wi-Fi', signal_strength: 95, security: 'Open', bssids: ['02:00:00:00:01:01'] },
    { id: 'shared-secured', ssid: 'Shared Wi-Fi', signal_strength: 70, security: 'WPA2', bssids: ['02:00:00:00:02:01', '02:00:00:00:02:02'] }
  ];
  const submissions: Array<{ssid: string; password?: string}> = [];
  await page.route('**/api/network/wifi/scan', route => route.fulfill({ json: networks }));
  await page.route('**/api/network/wifi/test', route => {
    submissions.push(route.request().postDataJSON());
    return route.fulfill({ status: 400, json: { error: 'Submission captured' } });
  });
  await page.goto('/setup/new/wifi');
  await expect(page.getByText('Online', { exact: true })).toBeVisible();
  const selector = page.getByRole('combobox', { name: 'SSID', exact: true });
  await selector.selectOption('shared-secured');
  await expect(selector.locator('option:checked')).toContainText('WPA2');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect.poll(() => submissions.length).toBe(1);
  expect(submissions[0].ssid).toBe('Shared Wi-Fi');
  expect(submissions[0].password).toBe('password');
  expect(submissions[0]).not.toHaveProperty('bssid');
  await selector.selectOption('shared-open');
  await expect(page.getByLabel('Password', { exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await expect.poll(() => submissions.length).toBe(2);
  expect(submissions[1].ssid).toBe('Shared Wi-Fi');
  expect(submissions[1]).not.toHaveProperty('password');
});


test('delete confirmation requires explicit approval and deletes only the selected profile', async ({ page }) => {
  let networks = [
    { id: 'wifi-profile', transport: 'wifi', display_name: 'Company Wi-Fi', managed: true },
    { id: 'wired-profile', transport: 'ethernet', display_name: 'Office Ethernet', managed: true }
  ];
  const deleted: string[] = [];
  await page.route('**/api/network/saved', route => route.fulfill({ json: networks }));
  await page.route('**/api/network/saved/*', route => {
    expect(route.request().method()).toBe('DELETE');
    const id = new URL(route.request().url()).pathname.split('/').at(-1)!;
    deleted.push(id);
    networks = networks.filter(network => network.id !== id);
    return route.fulfill({ status: 204 });
  });
  await page.goto('/');
  const opener = page.getByRole('button', { name: 'Delete', exact: true }).first();
  await opener.focus();
  await opener.press('Enter');
  const dialog = page.getByRole('alertdialog', { name: 'Delete connection?' });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText('Company Wi-Fi', { exact: true })).toBeVisible();
  await expect(dialog.getByText('If you’re using this network to reach the device, you may lose access to netsetup.', { exact: false })).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
  expect(deleted).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Delete', exact: true }).first()).toBeFocused();
  expect(deleted).toEqual([]);
  await page.getByRole('button', { name: 'Delete', exact: true }).first().click();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  expect(deleted).toEqual([]);
  await page.getByRole('button', { name: 'Delete', exact: true }).first().click();
  await dialog.getByRole('button', { name: 'Delete connection', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole('link', { name: /Company Wi-Fi/ })).not.toBeVisible();
  await expect(page.getByRole('link', { name: /Office Ethernet/ })).toBeVisible();
  expect(deleted).toEqual(['wifi-profile']);
});

test('delete confirmation keeps a failed deletion open and clears errors on cancellation', async ({ page }) => {
  const deleted: string[] = [];
  await page.route('**/api/network/saved', route => route.fulfill({ json: [
    { id: 'wifi-profile', transport: 'wifi', display_name: 'Company Wi-Fi', managed: true },
    { id: 'wired-profile', transport: 'ethernet', display_name: 'Office Ethernet', managed: true }
  ] }));
  await page.route('**/api/network/saved/*', route => {
    deleted.push(route.request().url().split('/').at(-1)!);
    return route.fulfill({ status: 409, json: { error: 'Another network operation is in progress' } });
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Delete', exact: true }).first().click();
  const dialog = page.getByRole('alertdialog', { name: 'Delete connection?' });
  await dialog.getByRole('button', { name: 'Delete connection', exact: true }).click();
  await expect(dialog.getByRole('alert')).toHaveText('Another network operation is in progress');
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.getByRole('button', { name: 'Delete', exact: true }).nth(1).click();
  await expect(dialog.getByText('Office Ethernet', { exact: true })).toBeVisible();
  await expect(dialog.getByRole('alert')).not.toBeVisible();
  expect(deleted).toEqual(['wifi-profile']);
});


test('page actions keep save visible and let the final report scroll above the bar', async ({ page }) => {
  await page.route('**/api/network/status/footer', route => route.fulfill({ json: { status: 'Connected', detail: report } }));
  await page.goto('/setup/new/wifi/result/footer');
  const bar = page.getByRole('region', { name: 'Page actions', exact: true });
  const save = bar.getByRole('button', { name: 'Save connection', exact: true });
  const finalText = page.getByText('Your previous connection has been restored.', { exact: false });
  for (const viewport of [{ width: 390, height: 844 }, { width: 320, height: 568 }]) {
    await page.setViewportSize(viewport);
    await page.evaluate(() => window.scrollTo(0, 0));
    await expect.poll(async () => {
      const button = await save.boundingBox();
      return !!button && button.y >= 0 && button.y + button.height <= viewport.height;
    }).toBe(true);
    await expect(bar.getByText('Settings not saved yet', { exact: true })).toBeVisible();
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    await expect.poll(async () => {
      await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
      const text = await finalText.boundingBox();
      const footer = await bar.boundingBox();
      return !!text && !!footer && text.y >= 0 && text.y + text.height <= footer.y;
    }).toBe(true);
    const primary = await save.boundingBox();
    const secondary = await bar.getByRole('button', { name: 'Change network' }).boundingBox();
    expect(primary!.x).toBeGreaterThan(secondary!.x);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(viewport.width);
  }
  await save.evaluate(element => { element.style.fontSize = '1.5rem'; });
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  await expect.poll(async () => {
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    const text = await finalText.boundingBox();
    const footer = await bar.boundingBox();
    return !!text && !!footer && text.y + text.height <= footer.y;
  }).toBe(true);
});

test('page actions reserve space for the last form field and expanded reset controls', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 568 });
  await page.route('**/api/network/wifi/scan', route => route.fulfill({ json: [] }));
  await page.route('**/api/network/saved', route => route.fulfill({ json: [] }));
  await page.goto('/setup/new/wifi');
  const bar = page.getByRole('region', { name: 'Page actions', exact: true });
  await page.getByRole('checkbox', { name: 'Enter network name manually' }).check();
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  await expect.poll(async () => {
    const input = await page.getByLabel('Custom DNS servers (comma separated)', { exact: true }).boundingBox();
    const footer = await bar.boundingBox();
    return !!input && !!footer && input.y >= 0 && input.y + input.height <= footer.y;
  }).toBe(true);
  await page.goto('/');
  await expect(bar).toBeVisible();
  await expect(page.getByText('Device is already online using automatic network settings (DHCP).', { exact: false })).toBeVisible();
  const initialHeight = (await bar.boundingBox())!.height;
  await bar.getByRole('button', { name: 'Reset all networks', exact: true }).click();
  await expect.poll(async () => (await bar.boundingBox())!.height).toBeGreaterThan(initialHeight);
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  await expect.poll(async () => {
    const text = await page.getByText('Device is already online using automatic network settings (DHCP).', { exact: false }).boundingBox();
    const footer = await bar.boundingBox();
    return !!text && !!footer && text.y >= 0 && text.y + text.height <= footer.y;
  }).toBe(true);
  await bar.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect.poll(async () => (await bar.boundingBox())!.height).toBe(initialHeight);
});

test('page actions release their reserved space on navigation and use the netsetup identity', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route('**/api/network/saved', route => route.fulfill({ json: [] }));
  await page.goto('/');
  await expect(page.locator('.device-identity')).toHaveText('netsetup • RS-12345');
  await expect(page).toHaveTitle('netsetup • RS-12345');
  await expect(page.getByRole('region', { name: 'Page actions', exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => parseFloat(document.documentElement.style.getPropertyValue('--netsetup-actions-height')))).toBeGreaterThan(0);
  await page.getByRole('button', { name: 'Add new network', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Choose connection type', exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => parseFloat(document.documentElement.style.getPropertyValue('--netsetup-actions-height')))).toBeGreaterThan(0);
  await page.getByRole('link', { name: 'Help', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'netsetup help', exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Page actions', exact: true })).not.toBeVisible();
  await expect.poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--netsetup-actions-height'))).toBe('');
  await expect(page).toHaveTitle('Help • netsetup • RS-12345');
});


test('page actions sit above the report on desktop and only pin on phones', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.route('**/api/network/status/desktop', route => route.fulfill({ json: { status: 'Connected', detail: report } }));
  await page.goto('/setup/new/wifi/result/desktop');
  const bar = page.getByRole('region', { name: 'Page actions', exact: true });
  const save = bar.getByRole('button', { name: 'Save connection', exact: true });
  await expect(save).toBeVisible();
  expect(await bar.evaluate(element => getComputedStyle(element).position)).toBe('static');
  const actions = (await bar.boundingBox())!;
  const summary = (await page.getByText('Full service connectivity', { exact: true }).boundingBox())!;
  expect(actions.y + actions.height).toBeLessThan(summary.y);
  expect(actions.width).toBeLessThan(1280);
  expect((await save.boundingBox())!.y).toBeLessThan(800);
  await expect.poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--netsetup-actions-height'))).toBe('0px');
  await page.setViewportSize({ width: 390, height: 844 });
  await expect.poll(() => bar.evaluate(element => getComputedStyle(element).position)).toBe('fixed');
  await expect.poll(() => page.evaluate(() => parseFloat(document.documentElement.style.getPropertyValue('--netsetup-actions-height')))).toBeGreaterThan(0);
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect.poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--netsetup-actions-height'))).toBe('0px');
  expect(await bar.evaluate(element => getComputedStyle(element).position)).toBe('static');
});
