import { test, expect, type Page, type Response } from '@playwright/test';

const admin = { username: 'admin', password: 'admin123' };
const pairEnabled = Boolean(
  (globalThis as { process?: { env?: Record<string, string> } }).process?.env?.PAIR_WHATSAPP
);
const maybeTest = pairEnabled ? test : test.skip;

async function login(page: Page) {
  await page.goto('/');
  await page.getByLabel('Username').fill(admin.username);
  await page.getByLabel('Password').fill(admin.password);
  await page.getByRole('button', { name: 'Sign In' }).click();
  await expect(page.getByText('Welcome, admin!')).toBeVisible();
}

async function goBusinessMode(page: Page) {
  await page.getByRole('button', { name: /Business Mode/i }).click();
  await expect(page.getByTitle('Settings')).toBeVisible();
}

async function goPersonalMode(page: Page) {
  await page.getByRole('button', { name: /Personal Mode/i }).click();
  await expect(page.getByTitle('New session')).toBeVisible();
}

test('login → business mode → settings visible', async ({ page }) => {
  await login(page);
  await goBusinessMode(page);

  await page.getByTitle('Settings').click();
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'WhatsApp Sessions' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'User Management' })).toBeVisible({ timeout: 20_000 });
});

test('create user in settings', async ({ page }) => {
  await login(page);
  await goBusinessMode(page);

  await page.getByTitle('Settings').click();
  await page.getByRole('button', { name: 'User Management' }).click();

  await page.getByRole('button', { name: 'Add User' }).click();

  const suffix = Date.now();
  const username = `agent_${suffix}`;
  const email = `agent_${suffix}@example.com`;

  await page.getByPlaceholder('Username').fill(username);
  await page.getByPlaceholder('Email').fill(email);
  await page.getByPlaceholder('Password').fill('agent123');
  await page.getByPlaceholder('Display Name (optional)').fill(`Agent ${suffix}`);
  await page.locator('select').selectOption('agent');

  await page.getByRole('button', { name: 'Create User' }).click();
  await expect(page.getByText(`Agent ${suffix}`)).toBeVisible({ timeout: 20_000 });
});

test('personal mode sync history triggers API', async ({ page }) => {
  await login(page);
  await goPersonalMode(page);

  // Create a session via API so we have one available for sync
  const token = await page.evaluate(() => localStorage.getItem('access_token'));
  await page.request.post('/api/whatsapp/connect', {
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    data: { session_name: `SyncTest-${Date.now()}` },
  });

  // Reload page so PersonalMode re-mounts and fetchSessions() picks up the new session
  await page.reload();
  await expect(page.getByTitle('New session')).toBeVisible();
  await page.waitForTimeout(2000);

  // Now trigger sync — the request should fire since we have a session
  const syncResponse = page.waitForResponse(
    (res: Response) =>
      res.url().includes('/api/chats/sync') && res.request().method() === 'POST',
    { timeout: 15_000 }
  );
  await page.getByTitle('Sync history').click();
  const res = await syncResponse;
  expect(res).toBeTruthy();
});

test('connect WhatsApp shows QR modal', async ({ page }) => {
  await login(page);
  await goBusinessMode(page);

  await page.getByTitle('Settings').click();
  await page.getByRole('button', { name: 'WhatsApp Sessions' }).click();

  await page.getByRole('button', { name: 'New Session' }).click();
  await expect(page.getByRole('heading', { name: 'Connect WhatsApp' })).toBeVisible();
  await expect(
    page.getByText(/Connecting to WhatsApp|Scan with WhatsApp on your phone|Connected!/)
  ).toBeVisible();
});

maybeTest('manual WhatsApp pairing (shows QR and waits)', async ({ page }) => {
  await login(page);
  await goBusinessMode(page);

  await page.getByTitle('Settings').click();
  await page.getByRole('button', { name: 'WhatsApp Sessions' }).click();
  await page.getByRole('button', { name: 'New Session' }).click();

  await expect(page.getByText('Connect WhatsApp')).toBeVisible();
  await expect(page.getByText('Scan with WhatsApp on your phone')).toBeVisible();

  // Keep browser open for manual QR pairing
  await page.pause();
});
