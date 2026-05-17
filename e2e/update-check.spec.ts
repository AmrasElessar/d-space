// SPDX-License-Identifier: GPL-3.0-or-later
//
// E2E #5: Güncelleme kontrol butonu — Sprint 3.6 + 3.5 smoke.
//
// `@tauri-apps/plugin-updater` çağrıları web kanalında reddeder (Tauri
// IPC plugin köprüsü yok). UI hata classification fallback'ini doğrula —
// kullanıcı "Beklenmedik hata" mesajını görür, app çökmez.

import { test, expect } from "@playwright/test";
import { buildTauriMockScript } from "./fixtures/tauri-mock";

test.describe("Update check (web kanal)", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(buildTauriMockScript({ showOnboarding: false }));
  });

  test("buton tıklanır, plugin yok hatası kullanıcıya friendly mesaj olarak çıkar", async ({
    page,
  }) => {
    await page.goto("/");
    const btn = page.locator(".update-notification .check-btn");
    await expect(btn).toBeVisible();
    await btn.click();
    // Hata fallback metni — i18n 'update.errorUnknown' veya errorNetwork
    await expect(btn).toContainText(/⚠/);
  });
});
