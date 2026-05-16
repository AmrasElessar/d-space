// SPDX-License-Identifier: GPL-3.0-or-later
//
// IndexSearchBar — Sprint 3.8 / Discovery #005 (Bölüm 5.6).
//
// vi.mock pattern UserRulesPanel/SnapshotPanel/DuplicatePanel ile aynı:
// `@tauri-apps/api/core`'un `invoke` fonksiyonu mocklanır, çağrı
// adı + parametreleri assert edilir.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import tr from "../locales/tr.json";

// invoke mock — modül seviyesinde tanımlanıp her test öncesi resetlenir.
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// Mock'tan sonra import — happy-dom hoist'i koruyalım.
import IndexSearchBar from "./IndexSearchBar.vue";

const i18n = createI18n({
  legacy: false,
  locale: "tr",
  messages: { tr },
});

function mountBar() {
  return mount(IndexSearchBar, {
    global: { plugins: [i18n] },
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  invokeMock.mockReset();
  // Default: index_status idle (admin var, indeks boş)
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === "index_status") {
      return Promise.resolve({
        volume_id: null,
        total_entries: 0,
        last_sync_unix: 0,
        mode: "idle",
      });
    }
    if (cmd === "index_search") {
      return Promise.resolve([]);
    }
    return Promise.reject(new Error(`Beklenmedik komut: ${cmd}`));
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("IndexSearchBar.vue", () => {
  it("monte olduğunda placeholder gösterir ve index_status çağrılır", async () => {
    const w = mountBar();
    await flushPromises();
    const input = w.find<HTMLInputElement>(
      '[data-testid="index-search-input"]',
    );
    expect(input.exists()).toBe(true);
    expect(input.attributes("placeholder")).toContain("Dosya adı");
    expect(invokeMock).toHaveBeenCalledWith("index_status", { drive: null });
  });

  it("boş query → IPC çağrılmaz, sonuç listesi boş", async () => {
    const w = mountBar();
    await flushPromises();
    const input = w.find<HTMLInputElement>(
      '[data-testid="index-search-input"]',
    );
    await input.setValue("");
    vi.advanceTimersByTime(200);
    await flushPromises();
    // index_status dışında çağrı olmamalı
    const searchCalls = invokeMock.mock.calls.filter(
      (c) => c[0] === "index_search",
    );
    expect(searchCalls.length).toBe(0);
    expect(w.find('[data-testid="index-search-results"]').exists()).toBe(false);
  });

  it("query → 150 ms debounce sonrası index_search çağrılır", async () => {
    invokeMock.mockImplementation((cmd: string, args: unknown) => {
      if (cmd === "index_status") {
        return Promise.resolve({
          volume_id: null,
          total_entries: 0,
          last_sync_unix: 0,
          mode: "idle",
        });
      }
      if (cmd === "index_search") {
        const a = args as { query: string; limit: number };
        return Promise.resolve([
          {
            volume_id: "\\\\.\\C:",
            file_ref: 42,
            parent_ref: 5,
            name: `${a.query}.txt`,
            full_path: null,
            attrs: 0,
          },
        ]);
      }
      return Promise.reject(new Error(`Beklenmedik: ${cmd}`));
    });

    const w = mountBar();
    await flushPromises();
    const input = w.find<HTMLInputElement>(
      '[data-testid="index-search-input"]',
    );

    await input.setValue("rapor");
    // Henüz debounce dolmamış
    expect(
      invokeMock.mock.calls.filter((c) => c[0] === "index_search").length,
    ).toBe(0);

    // 140 ms — hâlâ tetiklenmez
    vi.advanceTimersByTime(140);
    await flushPromises();
    expect(
      invokeMock.mock.calls.filter((c) => c[0] === "index_search").length,
    ).toBe(0);

    // 150 ms eşik — tetiklenir
    vi.advanceTimersByTime(20);
    await flushPromises();
    const searchCalls = invokeMock.mock.calls.filter(
      (c) => c[0] === "index_search",
    );
    expect(searchCalls.length).toBe(1);
    expect(searchCalls[0][1]).toEqual({ query: "rapor", limit: 50 });

    // Sonuç render edildi mi
    expect(w.find('[data-testid="index-search-results"]').exists()).toBe(true);
    expect(w.text()).toContain("rapor.txt");
  });

  it("hızlı arka arkaya yazımda yalnız son query gönderilir (debounce)", async () => {
    const w = mountBar();
    await flushPromises();
    const input = w.find<HTMLInputElement>(
      '[data-testid="index-search-input"]',
    );

    await input.setValue("r");
    vi.advanceTimersByTime(50);
    await input.setValue("ra");
    vi.advanceTimersByTime(50);
    await input.setValue("rap");
    vi.advanceTimersByTime(160);
    await flushPromises();

    const searchCalls = invokeMock.mock.calls.filter(
      (c) => c[0] === "index_search",
    );
    expect(searchCalls.length).toBe(1);
    expect(searchCalls[0][1]).toEqual({ query: "rap", limit: 50 });
  });

  it("sonuç tıklanınca update:selected-result emit edilir", async () => {
    const row = {
      volume_id: "\\\\.\\C:",
      file_ref: 99,
      parent_ref: 5,
      name: "selected.docx",
      full_path: "C:\\Users\\engin\\selected.docx",
      attrs: 0,
    };
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "index_status") {
        return Promise.resolve({
          volume_id: null,
          total_entries: 1,
          last_sync_unix: 1,
          mode: "ready",
        });
      }
      if (cmd === "index_search") {
        return Promise.resolve([row]);
      }
      return Promise.reject(new Error(cmd));
    });

    const w = mountBar();
    await flushPromises();
    const input = w.find<HTMLInputElement>(
      '[data-testid="index-search-input"]',
    );
    await input.setValue("sel");
    vi.advanceTimersByTime(200);
    await flushPromises();

    const rows = w.findAll(".result-row");
    expect(rows.length).toBe(1);
    await rows[0].trigger("click");
    const emitted = w.emitted("update:selected-result");
    expect(emitted).toBeTruthy();
    expect(emitted![0][0]).toMatchObject({
      file_ref: 99,
      name: "selected.docx",
    });
  });

  it("needs_admin mode → input devre dışı + uyarı placeholder", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "index_status") {
        return Promise.resolve({
          volume_id: null,
          total_entries: 0,
          last_sync_unix: 0,
          mode: "needs_admin",
        });
      }
      return Promise.resolve([]);
    });

    const w = mountBar();
    await flushPromises();
    const input = w.find<HTMLInputElement>(
      '[data-testid="index-search-input"]',
    );
    expect(input.attributes("disabled")).toBeDefined();
    expect(input.attributes("placeholder")).toContain("yönetici");
  });
});
