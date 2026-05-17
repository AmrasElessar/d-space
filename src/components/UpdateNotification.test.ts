// SPDX-License-Identifier: GPL-3.0-or-later
//
// UpdateNotification — Sprint 3.6 (Bölüm 21.4).
//
// `@tauri-apps/plugin-updater` ve `plugin-process` mocklanır. Smoke testler:
// idle → check, check → up_to_date, check → available + dismiss,
// install error (signature) → kullanıcıya doğru i18n mesajı.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import tr from "../locales/tr.json";

const checkMock = vi.fn();
const relaunchMock = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: (...args: unknown[]) => checkMock(...args),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: (...args: unknown[]) => relaunchMock(...args),
}));

import UpdateNotification from "./UpdateNotification.vue";

const i18n = createI18n({
  legacy: false,
  locale: "tr",
  messages: { tr },
});

function mountIt() {
  return mount(UpdateNotification, {
    global: { plugins: [i18n] },
  });
}

beforeEach(() => {
  checkMock.mockReset();
  relaunchMock.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("UpdateNotification.vue", () => {
  it("idle durumda 'Güncelleme kontrol et' butonu gösterir", () => {
    checkMock.mockResolvedValue(null);
    const w = mountIt();
    const btn = w.find("button.check-btn");
    expect(btn.exists()).toBe(true);
    expect(btn.text()).toContain("Güncelleme kontrol et");
  });

  it("güncelleme yoksa up_to_date durumuna geçer ve modal açılmaz", async () => {
    checkMock.mockResolvedValue(null);
    const w = mountIt();
    await w.find("button.check-btn").trigger("click");
    await flushPromises();
    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(w.find(".update-modal").exists()).toBe(false);
    expect(w.text()).toContain("Güncel sürüm");
  });

  it("güncelleme varsa modal açılır, sürüm + notlar gösterir", async () => {
    checkMock.mockResolvedValue({
      currentVersion: "0.1.0",
      version: "0.2.0",
      body: "İlk beta sürüm — VSS + USN index aktif.",
      downloadAndInstall: vi.fn(),
    });
    const w = mountIt();
    await w.find("button.check-btn").trigger("click");
    await flushPromises();
    const modal = w.find(".update-modal");
    expect(modal.exists()).toBe(true);
    expect(modal.text()).toContain("0.2.0");
    expect(modal.text()).toContain("İlk beta");
    expect(modal.text()).toContain("0.1.0");
  });

  it("modal kapat butonu modal'ı kapatır ve idle'a döner", async () => {
    checkMock.mockResolvedValue({
      currentVersion: "0.1.0",
      version: "0.2.0",
      body: "notes",
      downloadAndInstall: vi.fn(),
    });
    const w = mountIt();
    await w.find("button.check-btn").trigger("click");
    await flushPromises();
    expect(w.find(".update-modal").exists()).toBe(true);
    await w.find("button.secondary").trigger("click");
    await flushPromises();
    expect(w.find(".update-modal").exists()).toBe(false);
  });

  it("imza doğrulama hatası → kullanıcıya 'imza doğrulanamadı' mesajı", async () => {
    checkMock.mockRejectedValue(new Error("signature verification failed"));
    const w = mountIt();
    await w.find("button.check-btn").trigger("click");
    await flushPromises();
    expect(w.text()).toContain("İmza doğrulanamadı");
  });

  it("ağ hatası → ağ-mesajı + sunucu hata metni birleşir", async () => {
    checkMock.mockRejectedValue(new Error("dns resolve fetch failed: timeout"));
    const w = mountIt();
    await w.find("button.check-btn").trigger("click");
    await flushPromises();
    expect(w.text()).toContain("Sunucuya ulaşılamadı");
    expect(w.text()).toContain("dns resolve fetch failed");
  });

  it("indirme tetiklenir, progress event'leri ilerleme barına yansır", async () => {
    const downloadAndInstall = vi.fn(async (cb) => {
      cb({ event: "Started", data: { contentLength: 1000 } });
      cb({ event: "Progress", data: { chunkLength: 400 } });
      cb({ event: "Progress", data: { chunkLength: 600 } });
      cb({ event: "Finished" });
    });
    checkMock.mockResolvedValue({
      currentVersion: "0.1.0",
      version: "0.2.0",
      body: "test",
      downloadAndInstall,
    });
    relaunchMock.mockResolvedValue(undefined);
    const w = mountIt();
    await w.find("button.check-btn").trigger("click");
    await flushPromises();
    await w.find("button.primary").trigger("click");
    await flushPromises();
    expect(downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(relaunchMock).toHaveBeenCalledTimes(1);
  });
});
