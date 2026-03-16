import { test, expect } from '@playwright/test';

test('has title and can scan networks', async ({ page }) => {
  await page.goto('/');

  // Check title
  await expect(page.locator('h1', { hasText: 'Network Settings' })).toBeVisible();

  // Wait for the scanning to complete
  await expect(page.getByText('Home_Network').first()).toBeVisible({ timeout: 10000 });
  
  // Wait for Accordion to be initialized
  await page.waitForTimeout(500);

  // Click on a network to expand it. We use the trigger which handles the click.
  await page.locator('button', { hasText: 'Home_Network' }).first().click();
  
  // Find the expanded section
  const row = page.locator('.border-b').filter({ hasText: 'Home_Network' });
  
  // Wait for the animation to complete
  const input = row.getByPlaceholder('Network Password');
  await input.waitFor({ state: 'visible' });
  
  // Fill in password
  await input.fill('password');
  
  // Click connect
  await row.getByRole('button', { name: 'Connect' }).click();

  // Wait for success toast/message
  await expect(page.getByText('Successfully connected to Home_Network')).toBeVisible({ timeout: 10000 });
});
