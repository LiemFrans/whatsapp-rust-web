import { test, expect } from "@playwright/test";

test.describe("Helpdesk Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  /* ── Layout ─────────────────────────────────────── */

  test("renders the three-column layout", async ({ page }) => {
    await expect(page.locator(".sidebar")).toBeVisible();
    await expect(page.locator(".ticket-panel")).toBeVisible();
    await expect(page.locator(".conversation")).toBeVisible();
  });

  /* ── Sidebar ────────────────────────────────────── */

  test("sidebar shows logo and nav buttons", async ({ page }) => {
    await expect(page.locator(".sidebar-logo")).toHaveText("HD");
    const navButtons = page.locator(".sidebar-btn");
    await expect(navButtons).toHaveCount(4);
    await expect(navButtons.nth(0)).toContainText("Dashboard");
    await expect(navButtons.nth(1)).toContainText("Inbox");
    await expect(navButtons.nth(2)).toContainText("Broadcasts");
    await expect(navButtons.nth(3)).toContainText("Settings");
  });

  test("sidebar nav highlights active button on click", async ({ page }) => {
    const dashBtn = page.locator(".sidebar-btn", { hasText: "Dashboard" });
    await dashBtn.click();
    await expect(dashBtn).toHaveClass(/active/);
    // Inbox should no longer be active
    const inboxBtn = page.locator(".sidebar-btn", { hasText: "Inbox" });
    await expect(inboxBtn).not.toHaveClass(/active/);
  });

  /* ── Ticket panel ───────────────────────────────── */

  test("shows 'Conversations' heading and search input", async ({ page }) => {
    await expect(page.locator(".ticket-header h2")).toHaveText("Conversations");
    await expect(page.locator(".ticket-search input")).toBeVisible();
    await expect(page.locator(".ticket-search input")).toHaveAttribute(
      "placeholder",
      "Search conversations…"
    );
  });

  test("shows All / Open / Resolved tabs with counts", async ({ page }) => {
    const tabs = page.locator(".ticket-tab");
    await expect(tabs).toHaveCount(3);
    await expect(tabs.nth(0)).toContainText("All");
    await expect(tabs.nth(1)).toContainText("Open");
    await expect(tabs.nth(2)).toContainText("Resolved");
    // All tab should show total ticket count
    await expect(tabs.nth(0).locator(".count")).toHaveText("8");
  });

  test("All tab is active by default", async ({ page }) => {
    const allTab = page.locator(".ticket-tab").nth(0);
    await expect(allTab).toHaveClass(/active/);
  });

  test("renders all 8 tickets in the list by default", async ({ page }) => {
    const items = page.locator(".ticket-item");
    await expect(items).toHaveCount(8);
  });

  test("first ticket is selected by default", async ({ page }) => {
    const firstItem = page.locator(".ticket-item").first();
    await expect(firstItem).toHaveClass(/active/);
    await expect(firstItem.locator(".ticket-name")).toHaveText("Emma Thompson");
  });

  test("ticket items display name, preview, time, and status badge", async ({
    page,
  }) => {
    const firstItem = page.locator(".ticket-item").first();
    await expect(firstItem.locator(".ticket-name")).toBeVisible();
    await expect(firstItem.locator(".ticket-preview")).toBeVisible();
    await expect(firstItem.locator(".ticket-time")).toBeVisible();
    await expect(firstItem.locator(".badge").first()).toBeVisible();
  });

  test("priority badge is shown on priority tickets", async ({ page }) => {
    const firstItem = page.locator(".ticket-item").first();
    await expect(firstItem.locator(".badge-priority")).toHaveText("priority");
  });

  /* ── Tab filtering ──────────────────────────────── */

  test("Open tab filters to open tickets only", async ({ page }) => {
    await page.locator(".ticket-tab", { hasText: "Open" }).click();
    const items = page.locator(".ticket-item");
    // All visible items should have an 'open' badge
    const count = await items.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i++) {
      await expect(items.nth(i).locator(".badge-open")).toBeVisible();
    }
  });

  test("Resolved tab filters to resolved tickets only", async ({ page }) => {
    await page.locator(".ticket-tab", { hasText: "Resolved" }).click();
    const items = page.locator(".ticket-item");
    const count = await items.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i++) {
      await expect(items.nth(i).locator(".badge-resolved")).toBeVisible();
    }
  });

  test("switching back to All tab shows all tickets", async ({ page }) => {
    await page.locator(".ticket-tab", { hasText: "Open" }).click();
    await page.locator(".ticket-tab", { hasText: "All" }).click();
    await expect(page.locator(".ticket-item")).toHaveCount(8);
  });

  /* ── Ticket selection & conversation ────────────── */

  test("clicking a ticket updates the conversation header", async ({
    page,
  }) => {
    // Click the third ticket (Sarah Chen)
    await page.locator(".ticket-item").nth(2).click();
    await expect(page.locator(".conv-header-info h3")).toHaveText("Sarah Chen");
    await expect(page.locator(".conv-header-info span")).toContainText(
      "WhatsApp"
    );
  });

  test("clicking a ticket loads its messages", async ({ page }) => {
    // Default selection (Emma Thompson) should have 5 messages
    const messages = page.locator(".msg");
    await expect(messages).toHaveCount(5);
  });

  test("clicking a different ticket changes the messages", async ({
    page,
  }) => {
    // Click Sarah Chen (ticket index 2, id 3) — has 3 messages
    await page.locator(".ticket-item").nth(2).click();
    await expect(page.locator(".msg")).toHaveCount(3);
  });

  test("selected ticket gets active styling", async ({ page }) => {
    const thirdItem = page.locator(".ticket-item").nth(2);
    await thirdItem.click();
    await expect(thirdItem).toHaveClass(/active/);
    // First item should no longer be active
    await expect(page.locator(".ticket-item").first()).not.toHaveClass(
      /active/
    );
  });

  /* ── Conversation pane ──────────────────────────── */

  test("conversation header shows contact info and action buttons", async ({
    page,
  }) => {
    await expect(page.locator(".conv-header-avatar")).toBeVisible();
    await expect(page.locator(".conv-header-info h3")).toHaveText(
      "Emma Thompson"
    );
    // Action buttons
    await expect(
      page.locator(".conv-action-btn", { hasText: "Assign" })
    ).toBeVisible();
    await expect(
      page.locator(".conv-action-btn", { hasText: "Resolve" })
    ).toBeVisible();
  });

  test("messages show sender name, bubble, and timestamp", async ({
    page,
  }) => {
    const firstMsg = page.locator(".msg").first();
    await expect(firstMsg.locator(".msg-sender")).toBeVisible();
    await expect(firstMsg.locator(".msg-bubble")).toBeVisible();
    await expect(firstMsg.locator(".msg-time")).toBeVisible();
  });

  test("incoming messages have correct styling", async ({ page }) => {
    const incoming = page.locator(".msg.incoming").first();
    await expect(incoming).toBeVisible();
    await expect(incoming.locator(".msg-sender")).toHaveText("Emma Thompson");
  });

  test("outgoing messages have correct styling", async ({ page }) => {
    const outgoing = page.locator(".msg.outgoing").first();
    await expect(outgoing).toBeVisible();
    await expect(outgoing.locator(".msg-sender")).toHaveText("You");
  });

  test("Today divider is shown above messages", async ({ page }) => {
    await expect(page.locator(".msg-divider")).toHaveText("Today");
  });

  /* ── Composer ───────────────────────────────────── */

  test("composer has toolbar, textarea, and send button", async ({ page }) => {
    await expect(page.locator(".composer-toolbar")).toBeVisible();
    await expect(page.locator(".composer-textarea")).toBeVisible();
    await expect(page.locator(".composer-send")).toBeVisible();
  });

  test("composer toolbar has formatting buttons", async ({ page }) => {
    const tools = page.locator(".composer-tool");
    await expect(tools).toHaveCount(3);
  });

  test("composer textarea has correct placeholder", async ({ page }) => {
    await expect(page.locator(".composer-textarea")).toHaveAttribute(
      "placeholder",
      "Type your reply…"
    );
  });

  test("composer textarea accepts input", async ({ page }) => {
    const textarea = page.locator(".composer-textarea");
    await textarea.fill("Hello, how can I help?");
    await expect(textarea).toHaveValue("Hello, how can I help?");
  });

  /* ── Ticket with default messages ───────────────── */

  test("tickets without specific conversation show default messages", async ({
    page,
  }) => {
    // James Rodriguez (index 1, id 2) has no custom messages → defaults
    await page.locator(".ticket-item").nth(1).click();
    await expect(page.locator(".conv-header-info h3")).toHaveText(
      "James Rodriguez"
    );
    await expect(page.locator(".msg")).toHaveCount(2);
  });
});
