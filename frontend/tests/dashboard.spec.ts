/// <reference types="node" />
import { test, expect } from "@playwright/test";

// Clear localStorage before each test so persisted data doesn't leak between tests
test.beforeEach(async ({ page }) => {
  // Navigate first so we have a page context, then clear storage
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
});

/* ================================================================
   Routing
   ================================================================ */
test.describe("Routing", () => {
  test("default route loads the Inbox view", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator(".ticket-panel")).toBeVisible();
    await expect(page.locator(".conversation")).toBeVisible();
  });

  test("navigating to Dashboard shows analytics page", async ({ page }) => {
    await page.goto("/");
    await page.locator(".sidebar-btn", { hasText: "Dashboard" }).click();
    await expect(page.getByTestId("dashboard-page")).toBeVisible();
    await expect(page.locator(".ticket-panel")).not.toBeVisible();
  });

  test("navigating to Broadcasts shows broadcasts page", async ({ page }) => {
    await page.goto("/");
    await page.locator(".sidebar-btn", { hasText: "Broadcasts" }).click();
    await expect(page.getByTestId("broadcasts-page")).toBeVisible();
  });

  test("navigating to Settings shows settings page", async ({ page }) => {
    await page.goto("/");
    await page.locator(".sidebar-btn", { hasText: "Settings" }).click();
    await expect(page.getByTestId("settings-page")).toBeVisible();
  });

  test("can navigate from Dashboard back to Inbox", async ({ page }) => {
    await page.goto("/");
    await page.locator(".sidebar-btn", { hasText: "Dashboard" }).click();
    await expect(page.getByTestId("dashboard-page")).toBeVisible();
    await page.locator(".sidebar-btn", { hasText: "Inbox" }).click();
    await expect(page.locator(".ticket-panel")).toBeVisible();
  });

  test("active sidebar button gets highlighted on route change", async ({ page }) => {
    await page.goto("/");
    const inboxBtn = page.locator(".sidebar-btn", { hasText: "Inbox" });
    await expect(inboxBtn).toHaveClass(/active/);
    await page.locator(".sidebar-btn", { hasText: "Settings" }).click();
    const settingsBtn = page.locator(".sidebar-btn", { hasText: "Settings" });
    await expect(settingsBtn).toHaveClass(/active/);
    await expect(inboxBtn).not.toHaveClass(/active/);
  });

  test("direct URL navigation works for /dashboard", async ({ page }) => {
    await page.goto("/dashboard");
    await expect(page.getByTestId("dashboard-page")).toBeVisible();
    await expect(page.locator(".sidebar-btn", { hasText: "Dashboard" })).toHaveClass(/active/);
  });
});

/* ================================================================
   Inbox — Empty State
   ================================================================ */
test.describe("Inbox — Empty State", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.evaluate(() => localStorage.clear());
    await page.goto("/");
  });

  test("renders the three-column layout", async ({ page }) => {
    await expect(page.locator(".sidebar")).toBeVisible();
    await expect(page.locator(".ticket-panel")).toBeVisible();
    await expect(page.locator(".conversation")).toBeVisible();
  });

  test("sidebar shows logo and nav buttons", async ({ page }) => {
    await expect(page.locator(".sidebar-logo")).toHaveText("HD");
    await expect(page.locator(".sidebar-btn")).toHaveCount(4);
  });

  test("shows Conversations heading and search input", async ({ page }) => {
    await expect(page.locator(".ticket-header h2")).toHaveText("Conversations");
    await expect(page.locator(".ticket-search input")).toHaveAttribute("placeholder", "Search conversations…");
  });

  test("shows All / Open / Resolved tabs with zero counts", async ({ page }) => {
    const tabs = page.locator(".ticket-tab");
    await expect(tabs).toHaveCount(3);
    await expect(tabs.nth(0).locator(".count")).toHaveText("0");
    await expect(tabs.nth(1).locator(".count")).toHaveText("0");
    await expect(tabs.nth(2).locator(".count")).toHaveText("0");
  });

  test("ticket list shows empty state message", async ({ page }) => {
    await expect(page.getByTestId("ticket-list-empty")).toBeVisible();
    await expect(page.getByTestId("ticket-list-empty")).toContainText("No conversations yet");
  });

  test("no tickets are rendered", async ({ page }) => {
    await expect(page.locator(".ticket-item")).toHaveCount(0);
  });

  test("inbox empty state is shown when no ticket is selected", async ({ page }) => {
    await expect(page.getByTestId("inbox-empty")).toBeVisible();
    await expect(page.getByTestId("inbox-empty")).toContainText("No conversations yet");
    await expect(page.getByTestId("inbox-empty")).toContainText("Incoming WhatsApp messages");
  });

  test("sidebar avatar opens My Profile modal", async ({ page }) => {
    await page.locator(".sidebar-avatar").click();
    await expect(page.locator(".modal-header h3")).toHaveText("My Profile");
    await expect(page.locator(".modal-profile-name")).toHaveText("Frans D.");
  });
});

/* ================================================================
   Dashboard — Analytics
   ================================================================ */
test.describe("Dashboard — Analytics", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/dashboard");
  });

  test("shows dashboard page with header", async ({ page }) => {
    await expect(page.getByTestId("dashboard-page")).toBeVisible();
    await expect(page.locator(".dashboard-header h2")).toHaveText("Dashboard");
  });

  test("renders four stat cards", async ({ page }) => {
    await expect(page.getByTestId("stat-cards")).toBeVisible();
    await expect(page.locator(".stat-card")).toHaveCount(4);
  });

  test("connection status card shows a valid state", async ({ page }) => {
    const val = page.getByTestId("stat-connection-value");
    const text = await val.textContent();
    expect(["Disconnected", "Scanning QR…", "Connected"]).toContain(text?.trim());
  });

  test("total messages card shows 0 initially", async ({ page }) => {
    await expect(page.getByTestId("stat-messages-value")).toHaveText("0");
  });

  test("active conversations card shows 0 initially", async ({ page }) => {
    await expect(page.getByTestId("stat-conversations-value")).toHaveText("0");
  });

  test("uptime card shows 99.9%", async ({ page }) => {
    await expect(page.getByTestId("stat-uptime-value")).toHaveText("99.9%");
  });

  test("recent activity table is visible", async ({ page }) => {
    await expect(page.getByTestId("activity-table")).toBeVisible();
  });

  test("activity table shows empty state when no messages", async ({ page }) => {
    await expect(page.getByTestId("activity-empty")).toBeVisible();
    await expect(page.getByTestId("activity-empty")).toContainText("No messages yet");
  });
});

/* ================================================================
   Settings — WhatsApp QR Connection
   ================================================================ */
test.describe("Settings — QR Connection", () => {
  /**
   * Helper: wait until the Settings page connection state has stabilised.
   * The component always renders "Not connected" on mount, then the
   * useWhatsApp hook may sync to "connected" via REST / WS within a few
   * hundred ms.  We poll for up to 2 s — once the status text stops
   * changing for 300 ms we consider it settled and return the final value.
   */
  async function waitForSettledState(page: import("@playwright/test").Page) {
    await page.goto("/settings");
    await expect(page.getByTestId("settings-page")).toBeVisible();

    let lastText = "";
    let stableCount = 0;
    for (let i = 0; i < 20; i++) {
      const text =
        (await page.getByTestId("connection-status").textContent()) ?? "";
      if (text === lastText) {
        stableCount++;
        if (stableCount >= 3) break; // stable for 300ms
      } else {
        lastText = text;
        stableCount = 0;
      }
      await page.waitForTimeout(100);
    }
    return lastText;
  }

  test("shows settings page with connection card", async ({ page }) => {
    await waitForSettledState(page);
    await expect(page.getByTestId("settings-page")).toBeVisible();
    await expect(page.getByTestId("connection-card")).toBeVisible();
  });

  test("connection card shows a valid state", async ({ page }) => {
    const settled = await waitForSettledState(page);
    expect(["Not connected", "Waiting for scan…", "Connected"]).toContain(
      settled
    );
  });

  test("clicking Show QR Code enters scanning state (when disconnected)", async ({
    page,
  }) => {
    const settled = await waitForSettledState(page);
    if (settled !== "Not connected") {
      test.skip();
      return;
    }
    await page.getByTestId("connect-btn").click();
    await expect(page.getByTestId("connection-status")).toHaveText(
      "Waiting for scan…"
    );
    await expect(page.getByTestId("qr-scanning")).toBeVisible();
    await expect(page.locator(".qr-code-wrapper svg")).toBeVisible();
    await expect(page.getByTestId("simulate-btn")).toBeVisible();
  });

  test("clicking Simulate Connection enters connected state", async ({
    page,
  }) => {
    const settled = await waitForSettledState(page);
    if (settled !== "Not connected") {
      // Already connected — verify connected state directly
      await expect(page.getByTestId("connection-status")).toHaveText(
        "Connected"
      );
      return;
    }
    await page.getByTestId("connect-btn").click();
    await page.getByTestId("simulate-btn").click();
    await expect(page.getByTestId("connection-status")).toHaveText("Connected");
    await expect(page.getByTestId("qr-connected")).toBeVisible();
    await expect(page.getByTestId("qr-connected")).toContainText(
      "WhatsApp Device"
    );
    await expect(page.getByTestId("disconnect-btn")).toBeVisible();
    await expect(page.locator(".toast-msg")).toContainText(
      "connected successfully"
    );
  });

  test("disconnecting returns to initial state", async ({ page }) => {
    const settled = await waitForSettledState(page);
    if (settled !== "Not connected") {
      // Already connected — just disconnect
      await page.getByTestId("disconnect-btn").click();
      await expect(page.getByTestId("connection-status")).toHaveText(
        "Not connected"
      );
      await expect(page.locator(".toast-msg")).toContainText("disconnected");
      return;
    }
    await page.getByTestId("connect-btn").click();
    await page.getByTestId("simulate-btn").click();
    await expect(page.locator(".toast-msg")).toContainText(
      "connected successfully"
    );
    await page.locator(".toast-close").click();
    await page.getByTestId("disconnect-btn").click();
    await expect(page.getByTestId("connection-status")).toHaveText(
      "Not connected"
    );
    await expect(page.getByTestId("connect-btn")).toBeVisible();
    await expect(page.locator(".toast-msg")).toContainText("disconnected");
  });

  test("QR timer countdown is visible during scanning", async ({ page }) => {
    const settled = await waitForSettledState(page);
    if (settled !== "Not connected") {
      test.skip();
      return;
    }
    await page.getByTestId("connect-btn").click();
    await expect(page.locator(".qr-timer")).toBeVisible();
    await expect(page.locator(".qr-timer")).toContainText("QR refreshes in");
  });
});

/* ================================================================
   Broadcasts Page
   ================================================================ */
test.describe("Broadcasts Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/broadcasts");
  });

  test("shows compose card and campaigns table", async ({ page }) => {
    await expect(page.getByTestId("compose-card")).toBeVisible();
    await expect(page.getByTestId("campaigns-table")).toBeVisible();
  });

  test("campaigns table starts empty", async ({ page }) => {
    const rows = page.locator(".campaigns-table tbody tr");
    await expect(rows).toHaveCount(0);
  });

  test("sending a campaign without message shows validation toast", async ({ page }) => {
    await page.getByTestId("send-campaign-btn").click();
    await expect(page.locator(".toast-msg")).toHaveText("Please enter a message body");
  });

  test("sending a campaign without phone numbers shows validation toast", async ({ page }) => {
    await page.getByTestId("broadcast-message").fill("Hello world!");
    await page.getByTestId("send-campaign-btn").click();
    await expect(page.locator(".toast-msg")).toHaveText("Please enter at least one phone number");
  });

  test("sending a valid campaign adds a row to the table", async ({ page }) => {
    await page.getByTestId("broadcast-message").fill("Test broadcast message");
    await page.getByTestId("broadcast-phones").fill("+62 812-0001\n+62 812-0002\n+62 812-0003");
    await page.getByTestId("send-campaign-btn").click();
    // Table should now have 1 row
    await expect(page.locator(".campaigns-table tbody tr")).toHaveCount(1);
    // Toast should mention 3 recipients
    await expect(page.locator(".toast-msg")).toContainText("3 recipients");
    // Form should be cleared
    await expect(page.getByTestId("broadcast-message")).toHaveValue("");
    await expect(page.getByTestId("broadcast-phones")).toHaveValue("");
  });

  test("new campaign appears at the top of the table with 'queued' badge", async ({ page }) => {
    await page.getByTestId("broadcast-message").fill("New campaign");
    await page.getByTestId("broadcast-phones").fill("+1 555-0001");
    await page.getByTestId("send-campaign-btn").click();
    const firstRow = page.locator(".campaigns-table tbody tr").first();
    await expect(firstRow.locator(".campaign-badge")).toHaveText("queued");
    await expect(firstRow.locator(".td-message")).toContainText("New campaign");
  });
});
