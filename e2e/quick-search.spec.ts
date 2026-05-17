// SPDX-License-Identifier: GPL-3.0-or-later
//
// E2E #3: Hızlı arama (IndexSearchBar) — Sprint 3.5 smoke.

import { test, expect } from "@playwright/test";
import { buildTauriMockScript } from "./fixtures/tauri-mock";

test.describe("Quick search (IndexSearchBar)", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(buildTauriMockScript({ indexMode: "ready" }));
  });

  test("Ctrl+F arama kutusunu fokuslar, yazma debounce sonrası index_search çağırır", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.locator(".hero h1")).toBeVisible();
    await page.keyboard.press("Control+f");
    const input = page.locator('[data-testid="index-search-input"]');
    await expect(input).toBeFocused();
    await input.fill("rapor");
    // Debounce 150 ms — kısa bekleme + boş sonuç UI'ı doğrula
    await page.waitForTimeout(250);
    // Mock boş array döner → "Sonuç yok" görünür
    await expect(page.getByText(/Sonuç yok/i)).toBeVisible();
  });
});
