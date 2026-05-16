// SPDX-License-Identifier: GPL-3.0-or-later
//
// VolumeSidebar — Bölüm 15.1 v0.2 sol kolon (volume listesi + 3-kolon).
// `list_drives_cmd` IPC çağrısını mocklayıp render + selection emit'i
// doğrularız.

import { describe, expect, it, vi, beforeEach } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import tr from "../locales/tr.json";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import VolumeSidebar from "./VolumeSidebar.vue";

const i18n = createI18n({
  legacy: false,
  locale: "tr",
  messages: { tr },
});

function readyVolume(letter: string, label: string, total: number, free: number) {
  return {
    drive_letter: `${letter}:`,
    root_path: `${letter}:\\`,
    file_system: "NTFS",
    volume_label: label,
    volume_serial: 0x12345678,
    drive_kind: "Fixed",
    total_bytes: total,
    free_bytes: free,
    status: { kind: "Ready" },
    elapsed_ms: 4,
  };
}

function makeWrapper(selected = "C") {
  return mount(VolumeSidebar, {
    props: { selected },
    global: { plugins: [i18n] },
  });
}

describe("VolumeSidebar.vue", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("mount sırasında list_drives_cmd çağırır", async () => {
    invokeMock.mockResolvedValueOnce([
      readyVolume("C", "Windows", 1_000_000_000, 200_000_000),
    ]);
    const w = makeWrapper("C");
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("list_drives_cmd");
    expect(w.text()).toContain("C:");
    expect(w.text()).toContain("Windows");
  });

  it("seçili drive vurgulanır (drive-row-selected sınıfı)", async () => {
    invokeMock.mockResolvedValueOnce([
      readyVolume("C", "Windows", 1_000_000_000, 200_000_000),
      readyVolume("D", "Data", 2_000_000_000, 1_500_000_000),
    ]);
    const w = makeWrapper("D");
    await flushPromises();
    const rows = w.findAll(".drive-row");
    expect(rows).toHaveLength(2);
    expect(rows[0].classes()).not.toContain("drive-row-selected");
    expect(rows[1].classes()).toContain("drive-row-selected");
  });

  it("satır tıklaması update:selected emit eder", async () => {
    invokeMock.mockResolvedValueOnce([
      readyVolume("C", "Windows", 1_000_000_000, 200_000_000),
      readyVolume("E", "Backup", 500_000_000, 50_000_000),
    ]);
    const w = makeWrapper("C");
    await flushPromises();
    const rows = w.findAll(".drive-row");
    await rows[1].trigger("click");
    expect(w.emitted("update:selected")).toBeTruthy();
    expect(w.emitted("update:selected")![0]).toEqual(["E"]);
  });

  it("IPC hata durumunda error mesajı görünür", async () => {
    invokeMock.mockRejectedValueOnce({ kind: "Volume", message: "test fail" });
    const w = makeWrapper("C");
    await flushPromises();
    expect(w.find(".sidebar-err").exists()).toBe(true);
    expect(w.text()).toContain("test fail");
  });

  it("boş drive listesinde empty mesajı görünür", async () => {
    invokeMock.mockResolvedValueOnce([]);
    const w = makeWrapper("C");
    await flushPromises();
    expect(w.text()).toContain("Mount edilmiş sürücü bulunamadı.");
  });

  it("kullanım yüzdesi 90% üstü 'danger' sınıfı uygular", async () => {
    invokeMock.mockResolvedValueOnce([
      readyVolume("F", "Full", 1_000_000_000, 50_000_000), // %95 dolu
    ]);
    const w = makeWrapper("F");
    await flushPromises();
    const fill = w.find(".usage-fill");
    expect(fill.classes()).toContain("usage-danger");
  });
});
