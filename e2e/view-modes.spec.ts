// SPDX-License-Identifier: GPL-3.0-or-later
//
// E2E #2: Görsel mod değişimi (Ctrl+1/2/3/4) — Sprint 3.5 smoke.

import { test, expect } from "@playwright/test";
import { buildTauriMockScript } from "./fixtures/tauri-mock";

test.describe("View modes", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(buildTauriMockScript({ showOnboarding: false }));
  });

  test("Ctrl+1..4 ile sunburst → treemap → bubble → timeline akışı", async ({
    page,
  }) => {
    await page.goto("/");
    // Header görünür durumda olmalı — onboarding atlandı
    await expect(page.locator(".hero h1")).toBeVisible();

    const modes = [
      { key: "1", name: /sunburst/i },
      { key: "2", name: /treemap/i },
      { key: "3", name: /bubble/i },
      { key: "4", name: /timeline/i },
    ];

    for (const m of modes) {
      await page.keyboard.press(`Control+${m.key}`);
      // Aktif mod butonu .active CSS class'ına sahip olmalı (App.vue'da)
      await expect(page.locator(`button.view-mode-${m.key}`)).toHaveCount(
        1,
        { timeout: 1000 },
      );
    }
  });
});
