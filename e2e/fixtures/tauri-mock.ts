// SPDX-License-Identifier: GPL-3.0-or-later
//
// Tauri IPC mock — Sprint 3.5 web kanalı.
//
// Playwright `addInitScript` üzerinden enjekte edilir; `window.__TAURI_IPC__`
// override edilir, `@tauri-apps/api/core` `invoke` çağrıları deterministik
// JSON ile cevaplanır. Backend bağımsız akış testi.

export interface MockIpcOptions {
  /// `index_status` mode override (default "ready").
  indexMode?: "ready" | "idle" | "needs_admin" | "building" | "error";
  /// İlk açılışta onboarding görünsün mü (settings.onboarding_seen yoksa).
  showOnboarding?: boolean;
  /// `list_drives` çıktı override.
  drives?: Array<{ drive_letter: string; volume_label: string }>;
}

export function buildTauriMockScript(options: MockIpcOptions = {}): string {
  const opts = {
    indexMode: options.indexMode ?? "ready",
    showOnboarding: options.showOnboarding ?? false,
    drives: options.drives ?? [
      { drive_letter: "C", volume_label: "OS" },
      { drive_letter: "D", volume_label: "Data" },
    ],
  };

  return `
    (function () {
      const OPTS = ${JSON.stringify(opts)};
      const responses = {
        get_app_info: () => ({
          name: "D-Space",
          version: "0.2.0-beta",
          spec_version: "v1.4",
          spec_status: "DONDURULDU",
          license: "GPL-3.0-or-later",
          platform: "windows"
        }),
        check_privilege: () => ({ elevated: true, strategy: "MftService" }),
        list_drives_cmd: () => OPTS.drives.map(d => ({
          drive_letter: d.drive_letter,
          root_path: d.drive_letter + ":\\\\",
          file_system: "NTFS",
          volume_label: d.volume_label,
          volume_serial: 305419896,
          drive_kind: "Fixed",
          total_bytes: 1099511627776,
          free_bytes: 549755813888,
          status: { kind: "Ready" },
          elapsed_ms: 1
        })),
        get_setting_cmd: (args) => {
          if (args?.key === "onboarding_seen") return OPTS.showOnboarding ? null : "1";
          if (args?.key === "scan_mode") return "fast";
          return null;
        },
        set_setting_cmd: () => null,
        index_status: () => ({
          volume_id: "\\\\\\\\.\\\\C:",
          total_entries: OPTS.indexMode === "ready" ? 12345 : 0,
          last_sync_unix: Math.floor(Date.now() / 1000),
          mode: OPTS.indexMode
        }),
        index_search: () => [],
        list_staging: () => [],
        list_snapshots: () => [],
        get_db_info: () => ({
          path: "C:\\\\Users\\\\test\\\\AppData\\\\Local\\\\DSpace\\\\db\\\\dspace.sqlite",
          schema_version: 3,
          journal_mode: "wal",
          page_size: 4096,
          table_count: 11,
          spec_version: "v1.4"
        })
      };

      window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
      window.__TAURI_INTERNALS__.invoke = function(cmd, args) {
        const handler = responses[cmd];
        if (handler) {
          return Promise.resolve(handler(args));
        }
        return Promise.reject(new Error("[e2e mock] handler yok: " + cmd));
      };
      window.__TAURI_INTERNALS__.metadata = window.__TAURI_INTERNALS__.metadata || {
        windows: [{ label: "main" }],
        currentWindow: { label: "main" }
      };
    })();
  `;
}
