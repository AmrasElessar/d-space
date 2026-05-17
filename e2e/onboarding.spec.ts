// SPDX-License-Identifier: GPL-3.0-or-later
//
// E2E #1: Onboarding flow — Sprint 3.5 smoke (Bölüm 20.4).

import { test, expect } from "@playwright/test";
import { buildTauriMockScript } from "./fixtures/tauri-mock";

test.describe("Onboarding", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(
      buildTauriMockScript({ showOnboarding: true, indexMode: "idle" }),
    );
  });

  test("üç slide ilerler, mod seçimi sonrası kapanır", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Görmek")).toBeVisible();
    await page.getByRole("button", { name: /İleri/ }).click();
    await expect(page.getByText("Anlamak")).toBeVisible();
    await page.getByRole("button", { name: /İleri/ }).click();
    await expect(page.getByText("Geri kazanmak")).toBeVisible();
    await page.getByRole("button", { name: /Devam/ }).click();
    await expect(page.getByText(/Tarama Modu Seçimi/)).toBeVisible();
    await page.getByText(/Hızlı Mod/).click();
    await page.getByRole("button", { name: /Başla/ }).click();
    // Onboarding kapanmış olmalı — header görünür
    await expect(page.locator(".hero h1")).toBeVisible();
  });
});
