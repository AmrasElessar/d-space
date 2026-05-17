// SPDX-License-Identifier: GPL-3.0-or-later
//
// E2E #4: Tema cycle (auto → dark → light → auto) — Sprint 3.5 smoke.

import { test, expect } from "@playwright/test";
import { buildTauriMockScript } from "./fixtures/tauri-mock";

test.describe("Theme toggle", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(buildTauriMockScript({ showOnboarding: false }));
  });

  test("tema butonu üç state arasında döner", async ({ page }) => {
    await page.goto("/");
    const btn = page.locator(".hero .adv-toggle", { hasText: /Koyu|Açık|Sistem/ });
    await expect(btn).toBeVisible();
    const initial = await btn.textContent();
    await btn.click();
    const next = await btn.textContent();
    expect(next).not.toEqual(initial);
    await btn.click();
    const third = await btn.textContent();
    expect(third).not.toEqual(next);
  });
});
