import { test, expect } from "@playwright/test";

// ──────────────────────────────────────────────────────────────────────────────
// WhatsApp Web Clone – Playwright E2E Tests
//
// Each test mocks the backend API so tests run without a real Rust server.
// ──────────────────────────────────────────────────────────────────────────────

test.describe("WhatsApp Web Clone", () => {
  test.beforeEach(async ({ page }) => {
    await page.route("**/api/bootstrap", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          qr_code: "2@TestQRCodeData123,refkey,pubkey,clientid",
          is_connected: false,
          is_syncing: false,
          chats: [],
          contacts: [],
          logout_hint: null,
        }),
      });
    });

    await page.route("**/api/chats/**/messages", async (route) => {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ error: "Chat not found" }),
      });
    });
  });

  test("page loads successfully", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveTitle(/WhatsApp/);
  });

  test("QR code element renders on screen", async ({ page }) => {
    await page.goto("/");
    const qr = page.locator('[data-testid="qr-code"]');
    await expect(qr).toBeVisible({ timeout: 10_000 });
  });

  test("chat input and send button are visible when connected", async ({
    page,
  }) => {
    await page.route("**/api/bootstrap", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          qr_code: null,
          is_connected: true,
          is_syncing: false,
          chats: [],
          contacts: [],
          logout_hint: null,
        }),
      });
    });

    await page.goto("/");

    await page.locator('[data-testid="toggle-new-chat"]').click();
    await page.locator('[data-testid="phone-input"]').fill("15551234567");
    await page.locator('[data-testid="new-chat-button"]').click();

    const input = page.locator('[data-testid="message-input"]');
    const sendBtn = page.locator('[data-testid="send-button"]');
    await expect(input).toBeVisible({ timeout: 10_000 });
    await expect(sendBtn).toBeVisible();
  });

  test("sending a message shows success state", async ({ page }) => {
    await page.route("**/api/bootstrap", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          qr_code: null,
          is_connected: true,
          is_syncing: false,
          chats: [],
          contacts: [],
          logout_hint: null,
        }),
      });
    });

    await page.route("**/api/messages/send", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          success: true,
          message_id: "3EB0MOCK123456",
        }),
      });
    });

    await page.goto("/");

    await page.locator('[data-testid="toggle-new-chat"]').click();
    await page.locator('[data-testid="phone-input"]').fill("15551234567");
    await page.locator('[data-testid="new-chat-button"]').click();

    await page
      .locator('[data-testid="message-input"]')
      .fill("Hello from Playwright!");
    await page.locator('[data-testid="send-button"]').click();

    const status = page.locator('[data-testid="send-status"]');
    await expect(status).toContainText("sent", { timeout: 5_000 });

    await expect(page.locator('[data-testid="message-bubble"]')).toContainText(
      "Hello from Playwright!",
    );
  });
});
