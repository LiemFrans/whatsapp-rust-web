import { test, expect, type Page } from '@playwright/test';

const BASE_URL = 'http://localhost:5173';
const API_URL = 'http://localhost:8080';

// ─── Helper: Login ──────────────────────────────────────────────

async function login(page: Page, username = 'admin', password = 'admin123') {
  await page.goto('/login');
  await page.fill('input[name="username"], input[placeholder*="username" i], input[type="text"]', username);
  await page.fill('input[name="password"], input[placeholder*="password" i], input[type="password"]', password);
  await page.click('button[type="submit"]');
  // Wait for redirect to mode select
  await page.waitForURL('**/mode-select', { timeout: 10000 });
}

async function getAuthToken(): Promise<string> {
  const res = await fetch(`${API_URL}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: 'admin', password: 'admin123' }),
  });
  const data = await res.json();
  return data.access_token;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 1. Authentication Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Authentication', () => {
  test('should show login page when not authenticated', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL(/.*login/);
  });

  test('should login with valid credentials', async ({ page }) => {
    await login(page);
    await expect(page).toHaveURL(/.*mode-select/);
  });

  test('should reject invalid credentials', async ({ page }) => {
    await page.goto('/login');
    await page.fill('input[type="text"]', 'admin');
    await page.fill('input[type="password"]', 'wrongpassword');
    await page.click('button[type="submit"]');
    // Should stay on login page with error
    await expect(page).toHaveURL(/.*login/);
    await expect(page.locator('text=Invalid credentials').or(page.locator('[class*="error"]').or(page.locator('[class*="red"]')))).toBeVisible({ timeout: 5000 });
  });

  test('should redirect to login after logout', async ({ page }) => {
    await login(page);
    // Click logout button
    await page.click('button[title="Logout"]');
    await expect(page).toHaveURL(/.*login/, { timeout: 5000 });
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 2. Mode Selection Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Mode Selection', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('should display mode selection page with two modes', async ({ page }) => {
    await expect(page.locator('text=Personal').first()).toBeVisible();
    await expect(page.locator('text=Business').first()).toBeVisible();
  });

  test('should navigate to personal mode', async ({ page }) => {
    await page.click('text=Personal');
    await expect(page).toHaveURL(/.*personal/, { timeout: 5000 });
  });

  test('should navigate to business mode', async ({ page }) => {
    await page.click('text=Business');
    await expect(page).toHaveURL(/.*business/, { timeout: 5000 });
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 3. Personal Mode Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Personal Mode', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('text=Personal');
    await page.waitForURL('**/personal**', { timeout: 5000 });
  });

  test('should show chat list with demo chats', async ({ page }) => {
    // Wait for chat list to load
    await page.waitForTimeout(2000);
    // Should show demo contact names
    const chatItems = page.locator('[class*="cursor-pointer"], [class*="chat"]').filter({ hasText: /John|Jane|Bob|Alice|Team|Support/ });
    await expect(chatItems.first()).toBeVisible({ timeout: 10000 });
  });

  test('should show connected status indicator', async ({ page }) => {
    // The WiFi icon should be visible (connected)
    await expect(page.locator('[title="Connected"]').or(page.locator('svg.text-wa-green').first())).toBeVisible({ timeout: 10000 });
  });

  test('should select a chat and show messages', async ({ page }) => {
    await page.waitForTimeout(2000);
    // Click on first visible chat
    const chatItem = page.locator('[class*="cursor-pointer"], button').filter({ hasText: /John|Jane|Bob/ }).first();
    await chatItem.click();
    // Should show messages
    await page.waitForTimeout(1000);
    await expect(page.locator('[class*="chat-bg"], [class*="message"]').first()).toBeVisible({ timeout: 5000 });
  });

  test('should navigate to settings tab', async ({ page }) => {
    // Click settings icon in sidebar
    await page.click('button[title="Settings"]');
    await page.waitForTimeout(500);
    // Should show settings panel with Sessions section
    await expect(page.locator('text=Settings').first()).toBeVisible();
    await expect(page.locator('text=WhatsApp Sessions').first()).toBeVisible();
  });

  test('should navigate back to mode select', async ({ page }) => {
    await page.click('button[title="Back to mode select"]');
    await expect(page).toHaveURL(/.*mode-select/, { timeout: 5000 });
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 4. Business Mode Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Business Mode', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('text=Business');
    await page.waitForURL('**/business**', { timeout: 5000 });
  });

  test('should show queue panel by default', async ({ page }) => {
    await expect(page.locator('text=Queue').first()).toBeVisible();
  });

  test('should show analytics data', async ({ page }) => {
    await page.click('button[title="Analytics"]');
    await page.waitForTimeout(1000);
    await expect(page.locator('text=Analytics').first()).toBeVisible();
    await expect(page.locator('text=Messages Today').first()).toBeVisible();
  });

  test('should show tickets panel', async ({ page }) => {
    await page.click('button[title="Tickets"]');
    await page.waitForTimeout(1000);
    await expect(page.locator('text=Tickets').first()).toBeVisible();
  });

  test('should show quick replies panel', async ({ page }) => {
    await page.click('button[title="Quick Replies"]');
    await page.waitForTimeout(1000);
    await expect(page.locator('text=Quick Replies').first()).toBeVisible();
    // Should show seeded quick replies
    await expect(page.locator('text=/hi').or(page.locator('text=Greeting')).first()).toBeVisible({ timeout: 5000 });
  });

  test('should show my chats panel', async ({ page }) => {
    await page.click('button[title="My Chats"]');
    await page.waitForTimeout(1000);
    await expect(page.locator('text=My Chats').first()).toBeVisible();
  });

  test('should navigate to settings', async ({ page }) => {
    await page.click('button[title="Settings"]');
    await page.waitForTimeout(1000);
    await expect(page.locator('text=Settings').first()).toBeVisible();
    await expect(page.locator('text=WhatsApp Sessions').first()).toBeVisible();
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 5. Settings & User Management Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Settings & User Management', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('text=Business');
    await page.waitForURL('**/business**', { timeout: 5000 });
    await page.click('button[title="Settings"]');
    await page.waitForTimeout(1000);
  });

  test('should show WhatsApp sessions section', async ({ page }) => {
    await expect(page.locator('text=WhatsApp Sessions').first()).toBeVisible();
    // Should show the demo session
    await expect(page.locator('text=Demo Session').or(page.locator('text=connected').first())).toBeVisible({ timeout: 5000 });
  });

  test('should show user management section for admin', async ({ page }) => {
    // Click on User Management tab
    await page.click('text=User Management');
    await page.waitForTimeout(1000);
    // Should show users list
    await expect(page.locator('text=admin').first()).toBeVisible();
    await expect(page.locator('text=agent1').or(page.locator('text=Agent One')).first()).toBeVisible({ timeout: 5000 });
  });

  test('should show my profile section', async ({ page }) => {
    await page.click('text=My Profile');
    await page.waitForTimeout(500);
    await expect(page.locator('text=@admin').first()).toBeVisible();
    await expect(page.locator('text=admin@localhost').first()).toBeVisible();
  });

  test('should open create user form', async ({ page }) => {
    await page.click('text=User Management');
    await page.waitForTimeout(500);
    await page.click('text=Add User');
    await page.waitForTimeout(500);
    await expect(page.locator('input[placeholder="Username"]').first()).toBeVisible();
    await expect(page.locator('input[placeholder="Email"]').first()).toBeVisible();
    await expect(page.locator('input[placeholder="Password"]').first()).toBeVisible();
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 6. WhatsApp Pairing / Session Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('WhatsApp Session', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('text=Personal');
    await page.waitForURL('**/personal**', { timeout: 5000 });
  });

  test('should open QR code modal when creating new session', async ({ page }) => {
    // Click New Session button
    await page.click('button[title="New session"]');
    await page.waitForTimeout(1000);
    // QR modal should appear
    await expect(page.locator('text=Connecting').or(page.locator('text=Scan QR Code')).first()).toBeVisible({ timeout: 5000 });
  });

  test('should show session connected after simulated QR scan', async ({ page }) => {
    await page.click('button[title="New session"]');
    // Wait for simulated connection (5 second timeout)
    await page.waitForTimeout(7000);
    // Session should show connected
    await expect(page.locator('[title="Connected"]').or(page.locator('svg.text-wa-green').first())).toBeVisible({ timeout: 5000 });
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 7. Chat Sync Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Chat Sync', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.click('text=Personal');
    await page.waitForURL('**/personal**', { timeout: 5000 });
  });

  test('should have sync button and trigger sync', async ({ page }) => {
    // The sync/refresh button should be visible
    await expect(page.locator('button[title="Sync history"]').first()).toBeVisible();
    // Click sync button - should not crash
    await page.click('button[title="Sync history"]');
    await page.waitForTimeout(2000);
    // Page should still be functional
    await expect(page.locator('text=Chats').first()).toBeVisible();
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 8. API Integration Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('API Integration', () => {
  let token: string;

  test.beforeAll(async () => {
    token = await getAuthToken();
  });

  test('GET /api/health returns ok', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/health`);
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.status).toBe('ok');
  });

  test('GET /api/auth/me returns user info', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/auth/me`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.user.username).toBe('admin');
    expect(data.user.role).toBe('admin');
  });

  test('GET /api/chats returns demo chats', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/chats`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.chats.length).toBeGreaterThanOrEqual(6);
  });

  test('GET /api/whatsapp/sessions returns sessions', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/whatsapp/sessions`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.sessions.length).toBeGreaterThanOrEqual(1);
  });

  test('GET /api/users returns user list', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/users`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.users.length).toBeGreaterThanOrEqual(2);
  });

  test('GET /api/business/queue returns queue', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/business/queue`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(Array.isArray(data.queue)).toBeTruthy();
  });

  test('GET /api/business/analytics returns analytics', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/business/analytics`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data).toHaveProperty('messages_today');
    expect(data).toHaveProperty('unassigned_count');
  });

  test('GET /api/business/quick-replies returns quick replies', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/business/quick-replies`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.quick_replies.length).toBeGreaterThanOrEqual(3);
  });

  test('GET /api/business/agents returns agent list', async ({ request }) => {
    const res = await request.get(`${API_URL}/api/business/agents`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.agents.length).toBeGreaterThanOrEqual(2);
  });

  test('POST /api/chats/:id/messages returns messages', async ({ request }) => {
    // First get chats
    const chatsRes = await request.get(`${API_URL}/api/chats`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    const chats = await chatsRes.json();
    const chatId = chats.chats[0].id;

    const res = await request.get(`${API_URL}/api/chats/${chatId}/messages`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.messages.length).toBeGreaterThanOrEqual(1);
  });
});

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 9. Dark Mode Test
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

test.describe('Dark Mode', () => {
  test('should toggle dark mode', async ({ page }) => {
    await login(page);
    await page.click('text=Personal');
    await page.waitForURL('**/personal**', { timeout: 5000 });

    // Click dark mode toggle
    const darkToggle = page.locator('button[title="Dark mode"]').or(page.locator('button[title="Light mode"]')).first();
    await darkToggle.click();
    await page.waitForTimeout(500);

    // HTML should have dark class
    const hasDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(hasDark).toBeTruthy();

    // Toggle back
    await darkToggle.click();
    await page.waitForTimeout(500);
    const hasDark2 = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(hasDark2).toBeFalsy();
  });
});
