<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# D-Space — Sonraki Oturum Yol Planı

> **Son güncelleme:** 2026-05-17 13:55 — v0.2.0-beta polish dalgası tamam
> **Şu anki sürüm:** `v0.2.0-beta` adayı — pubkey aktif, 15 commit ahead origin, tag çekmeye hazır
> **Hedef:** `v0.2.0-beta` → `v0.3.0` → `v1.0.0-stable`
> **Test sayısı:** 124 Rust + 34 frontend = **158 passing** (+ 9 vss-gated)
> **2026-05-17 polish dalgası:** info popover, 3-kolon layout, tarama durdur/iptal, disk hardware rozeti, hard link dedup, sunburst 3D + tıklama detayı, tema pill renkleri light tema uyumlu, canlı log scroll + tooltip

Bu doküman bir sonraki oturum başlar başlamaz açılacak. Akış:
1. Git pull + son durumu doğrula
2. Aşağıdaki **sıradaki sprint** ile devam et
3. Tamamladığında bu dokümanın "ŞU AN" kısmını güncelle

---

## 🟢 ŞU AN: v0.2.0-beta tag — push + tag bekliyor

**Updater pubkey aktif** (commit 2714c79), private/password Secrets'ta.
**Kalan adımlar** (kullanıcı eylemi):

1. **Origin push** (15 commit bekliyor)
   ```powershell
   git push origin main
   ```
2. **Tag + release**
   ```powershell
   git tag v0.2.0-beta && git push origin v0.2.0-beta
   ```
   `.github/workflows/release.yml` MSI/NSIS + latest.json üretir.

> **2026-05-17 polish dalgası özet:** kullanıcı tema/dil/UX feedback'i ile
> info popover pattern, 3-kolon dense layout, disk hardware probe (IOCTL),
> hard link dedup (1 TB diskte 1.2 TB raporu fix), tarama durdur/iptal,
> canlı log scroll + bold filename, sunburst 3D + click detay, tüm
> pill'lerin light tema uyumu. Tüm değişiklikler v0.2.0-beta'ya dahil.

### Sprint 3.1c — Gerçek üç-kolon (sol volume sidebar) ✅ TAMAMLANDI (2026-05-16)
- **Sonuç:** App grid `280px minmax(0, 1fr)` — sticky sol sidebar + akıcı orta workspace; workspace içindeki 2-kolon (SnapshotPanel + DuplicatePanel/Detail) korundu, görsel olarak toplam 3-kolon.
- **Dosyalar:**
  - `src-tauri/src/volume/enumerate.rs` (yeni, ~80 satır) — Win32 `GetLogicalDrives` bitmask → `list_drives()` her sürücü için `pre_flight_check`.
  - `src-tauri/src/lib.rs` — `list_drives_cmd` Tauri command (spawn_blocking).
  - `src/components/VolumeSidebar.vue` (yeni, ~440 satır) — drive listesi + kullanım barı + status pill + i18n.
  - `src/App.vue` — `.app-frame` grid wrapper, `onSidebarDriveSelect` handler, drive değişiminde otomatik pre-flight.
  - `src/locales/{tr,en}.json` — `sidebar.*` anahtarları (refresh/empty/loading/status/kind).
- **Test:** 6 yeni frontend test (mount IPC, selection, emit, error, empty, usage class) + 2 yeni Rust test (enumerate sıralama, list_drives smoke) → **76 Rust + 16 frontend = 92 test passing**.
- **Responsive:** `<960px` sidebar üste kayar, akıcı tek kolon.

### Sprint 2H.3 — VSS pool (Bölüm 34 v0.2) ✅ TAMAMLANDI (2026-05-15, `f87979e`)
- **Sonuç:** Plan A — `winapi 0.3.9` crate'i `IVssBackupComponents` + factory'yi sağladı, manuel vtable gerekmedi. Discovery Log #002 → çözüldü, #004 yeni revize (`VSS_CTX_FILE_SHARE_BACKUP` writer-less).
- **Dosyalar:**
  - `src-tauri/src/locked_file/vss.rs` (563 satır) — düşük seviye COM köprüsü, `SnapshotProvider` trait (mock-friendly).
  - `src-tauri/src/locked_file/vss_pool.rs` (511 satır) — worker thread, per-volume dedupe, lease renewal + reaper.
  - `src-tauri/src/duplicate/scan.rs` — `hash_file_with_retry` (`ERROR_SHARING_VIOLATION` → VSS reader).
- **Feature gate:** `vss` (default OFF). Default build sıfır etki.
- **Test:** 9 yeni unit test (hresult mapping, BSTR roundtrip, snapshot_path, pool dedupe, lease drop, reaper eviction + scan integration) — `cargo test --features vss --lib` ile 83/83 geçer.

### Sprint 3.7 — Canlı tarama görseli (Live Sunburst) ✅ TAMAMLANDI (2026-05-16, `9d10e73`)
- **Sonuç:** `ScanProgress` overlay artık split layout — sol `LiveSunburst` (360×360 saf SVG arc, 2 halka, CSS transition), sağ sayısal stats. Backend `ScanProgress.partial_tree: Option<Vec<PartialNode>>` her 10k entry'de doluyor. `scan_full` "done" event'i final partial_tree taşır (root + 2 seviye + top-200).
- **Dosyalar:** `scan/tree.rs` (+`build_partial_view`+`PartialNode`), `scan/walk.rs` + `scan/find_first.rs` (10k checkpoint), `lib.rs` (done emit), `LiveSunburst.vue` (yeni, ~310 satır), `ScanProgress.vue` (split + `latestPartialTree` flicker önleme + mobile @media), i18n `scanProgress.liveEmpty`.
- **Test:** +4 Rust (`build_partial_view`: top-N + max_depth + empty + top-1) + +5 frontend (`LiveSunburst.test.ts`: empty, root=0, wedge count, largest largeArc, depth=2). 80 Rust + 21 frontend.

### Sprint 3.8 — USN Journal Index (Discovery #005) ✅ TAMAMLANDI (2026-05-16, `1ae1ad9`)
- **Sonuç:** Yeni `src-tauri/src/index/` modülü (mod/usn/persist/delta/watcher). SQLite migration `0003_usn_index.sql` (`usn_index` + `usn_watermark` + 2 index). 3 Tauri command: `index_build` / `index_status` / `index_search`. Frontend `IndexSearchBar.vue` sticky üst kutu, 150 ms debounce, top-50 sonuç. App.vue entegrasyon: Ctrl+F handler → IndexSearchBar input fokusu; startup'ta arka plan `index_build` (admin yoksa `needs_admin` döner, sessizce geçilir); `onIndexResultSelect` → minimal pseudo-node detay paneline.
- **v0.1 stub:** `index_build` gerçek `FSCTL_ENUM_USN_DATA` walker yerine **iskelet** (status döner). Baseline binary stream walker + race testleri sonraki minör sprint'te. Persist/delta/wraparound/watcher tam test edilmiş.
- **Test:** +25 Rust (USN parse_v2 + apply_delta upsert/delete + detect_wraparound + persist roundtrip + index_search + watcher shutdown) + +6 frontend (mount, empty, debounce single+rapid, click emit, needs_admin). 105 Rust + 27 frontend = **132 passing**.
- **Discovery Log #005** detaylı yazıldı: FSCTL kodları (`0x000900B3` / `0x000900BB`), USN_RECORD_V2 layout, windows-rs 0.61 const eksikliği (hand-roll), wraparound semantiği, migration 0003 sapması.

### Sprint 3.8.1 — USN baseline walker ✅ TAMAMLANDI (2026-05-17)
- **Sonuç:** `src-tauri/src/index/baseline.rs` (~290 src + ~315 test); saf
  parser helper'lar (`parse_journal_data`, `parse_enum_buffer`,
  `build_enum_request`, `records_to_entries`, `apply_baseline_batch`) +
  `cfg(windows)` IO (`enumerate_volume_baseline`): `CreateFileW(\\.\X:)` +
  `FSCTL_QUERY_USN_JOURNAL` + `FSCTL_ENUM_USN_DATA` döngüsü 1 MB tampon,
  ERROR_HANDLE_EOF EOF; bitince watermark yazılır.
- **Cargo Win32_System_IO** eklendi (DeviceIoControl).
- **`lib.rs::index_build`** stub değiştirildi → gerçek walker.
- **Test:** +12 (parse + request layout + apply_baseline_batch + race
  idempotency + watermark roundtrip). 117 Rust toplam.

### Sprint 3.5 — Playwright e2e altyapı ✅ SCAFFOLD (2026-05-17)
- **Sonuç:** `@playwright/test` dev dep + `playwright.config.ts` (web +
  webdriver iki proje) + `e2e/fixtures/tauri-mock.ts` (Tauri IPC stub) +
  5 smoke spec (onboarding, view-modes, quick-search, theme-toggle,
  update-check). `.github/workflows/e2e.yml` ubuntu web kanalı,
  `continue-on-error: true`.
- **v0.2.0 sonrası iş:** `webdriver` projesini `cargo install
  tauri-driver` + `pnpm tauri build --debug` ile aktive et; e2e CI gate'i
  required'a taşı. Şu an mock stub'ları gerçek IPC akışını birebir
  yansıtmıyor — `__TAURI_INTERNALS__` küçük tweak gerekebilir.

### Sprint 3.6 — Tauri updater gerçek ✅ TAMAMLANDI (2026-05-17)
- **Sonuç:** `tauri-plugin-updater` + `tauri-plugin-process` etkin;
  `tauri.conf.json` plugins.updater GitHub Releases latest.json
  endpoint'ine bağlı; release workflow `includeUpdaterJson: true` +
  `TAURI_SIGNING_PRIVATE_KEY` secrets passthrough.
- **`src/components/UpdateNotification.vue`** header butonu + modal
  (sürüm + notlar + indirme progress) → `relaunch()`. Hata tip
  ayrımı (imza/ağ/diğer) i18n. +7 vitest test.
- **Pubkey:** PLACEHOLDER (`REPLACE_BEFORE_RELEASE`). Maintainer
  `pnpm tauri signer generate` ile üretmeli — RELEASE_CHECKLIST.md §2.2.

---

## 🟡 v0.2.0-beta → v0.3.0 yolu (6 sprint)

### Faz 4 — Production hardening

| Sprint | Kapsam | Boyut |
|---|---|---|
| **4.1** Duplicate v0.2 | Head-hash prefilter (ilk 4KB) + rayon paralel hash + dup-aware sunburst overlay + Criterion bench (100GB <60sn hedef Bölüm 7.4) | ~400 satır |
| **4.2** Snapshot olgun | Retention cleanup (90gün, Bölüm 8.4) + streaming delta loader (Bölüm 9.6.5 — büyük snapshot karşılaştırması parça parça UI'a iletir) | ~300 satır |
| **4.3** Test piramidi | Synthetic NTFS VHDX fixture generator + Bölüm 20.3 critical test cases (1M+ düğüm sunburst render, %100 dolu disk 4-katman fallback, migration rollback). 200+ Rust + 150+ TS test hedefi | ~500 satır |
| **4.4** Network scanner | UNC path scanner (Bölüm 26.2 `NetworkShareScanner` trait gerçek impl) + bandwidth-aware streaming + partial result | ~250 satır |
| **4.5** LOD + animation | Bölüm 9.3 viewport-aware detail level (1M+ düğümde min_size threshold dinamik) + Bölüm 9.4 Vue transition standartları (sunburst rotate, treemap morph) | ~200 satır |
| **4.6** ReFS gerçek scanner | Bölüm 5.5 ReFS volume için optimize fallback (MFT yok, FindFirstFile + ReFS metadata) | ~150 satır |

**Tahmin:** 3-4 oturum.

---

## 🟢 v0.3.0 → v1.0.0-stable yolu (4 sprint)

### Faz 5 — Release polish

| Sprint | Kapsam |
|---|---|
| **5.1** Tray Live Monitor | Bölüm 13.2 polling interval (default 30dk) + 13.3 auto-clean premium (opt-in checkbox + battery throttle GetSystemPowerStatus) |
| **5.2** Competitive benchmark | Bölüm 37 4b ekranı — onboarding adım 4b'de "Senin C: vs WizTree/TreeSize/WinDirStat" görsel karşılaştırma (Bölüm 37.2 etik sınırlar) |
| **5.3** UX polish | Bölüm 15.4 animasyonlu illüstrasyon empty state + Bölüm 12.4 DoD 5220.22-M wipe (hassas dosya modu, opt-in) |
| **5.4** Production signing | EV cert satın alma (~$400/y) + signtool workflow entegrasyon + winget manifest + Scoop bucket (Bölüm 21.3) + SmartScreen reputation request |

**Tahmin:** 2-3 oturum.

---

## 🌌 Post-v1.0 (v2.0 — kapsam dışı, sonraki yıl)

- **Bölüm 5.2A Katman 3** — MFT Service Windows Service (kullanıcı için tek seferlik kurulum)
- **Bölüm 6.5 Tier 3** — Cloud ML inference (premium, opt-in)
- **Bölüm 6.5 model train** — TFLite gerçek model eğitimi (path/ext/size/age features → 0-100 score)
- **Bölüm 26.3** — Linux/macOS port (`CrossPlatformVolumeReader` trait gerçek impl)
- **Bölüm 26.4** — Plugin sistemi (WASM sandbox)
- **Bölüm 26.5** — Cloud Backup endpoint (D-Space Cloud + S3-compatible)
- **Bölüm 27.4 v1.0+** — Üçüncü taraf pen-test + ISO 27001 hazırlığı

---

## 📋 Her oturum başlangıç ritüeli

```bash
# 1. Origin'i pull et
cd C:/Projeler/d-space && git pull origin main

# 2. CI durumunu kontrol et
gh run list --limit 3 --workflow ci.yml

# 3. Tag'leri kontrol et
git tag -l --sort=-creatordate | head -5

# 4. Bu dokümandaki "ŞU AN" sırasındaki sprint'i aç

# 5. Sprint sonu:
#    - cargo test --lib + cargo clippy + cargo fmt --check
#    - pnpm test + pnpm build
#    - commit + push
#    - bu dokümanı güncelle
```

---

## 🔑 Önemli referanslar

- **Master spec:** `D-Space-Mimari-v1.4.docx` (37 bölüm, DONDURULDU)
- **Discovery Log:** `docs/DISCOVERY_LOG.md` (#001-004)
- **Threat Model:** `docs/THREAT_MODEL.md`
- **Release Checklist:** `docs/RELEASE_CHECKLIST.md`
- **CHANGELOG:** `CHANGELOG.md`
- **REPO_STANDARDS:** `.github/REPO_STANDARDS.md`
- **Memory (Claude):** `C:\Users\engin.okay\.claude\projects\C--Projeler-d-space\memory\`

## 🎯 v1.0 başarı kriterleri

- [ ] 200+ Rust test + 150+ frontend test (şu an 105 default + 9 vss-gated + 27 frontend = 141)
- [ ] Tüm 5 pillar v1 kapsamında (şu an alpha düzeyi)
- [ ] EV cert imzalı MSI + NSIS
- [ ] SmartScreen reputation kazanılmış
- [ ] VirusTotal 0/70 false positive
- [ ] Auto-updater çalışıyor (stable kanal)
- [ ] winget + Scoop'ta yayın
- [ ] CHANGELOG her release detaylı
- [ ] Discovery Log canlı (her sprint sonu güncel)
- [ ] Threat Model güncel (mitigasyonlar uygulanmış)

---

## 🚦 Sıradaki sprint kararı: **Faz 4.1 Duplicate v0.2** (önerilen)

v0.2.0-beta tag adayı hazır (3.8.1 + 3.6 + 3.5 scaffold tamam, 2026-05-17).
Maintainer pubkey gen + push + tag çekince beta yayınlanır. Sonraki
oturum **Faz 4** başlar.

**Önerilen sıra:**
1. **4.1 Duplicate v0.2** — head-hash prefilter + rayon paralel. En çok
   görünür kazanım (100 GB <60 sn hedef).
2. **4.3 Test piramidi** — synthetic NTFS VHDX + critical edge cases. 
   200+ Rust test hedefine doğru.
3. **4.2 Snapshot retention** + **4.5 LOD/animation** + **4.4 Network
   scanner** + **4.6 ReFS** — paralelleştirilebilir.

Alternatif (kullanıcı bir polish dalgası ister):
* **5.1 Tray live monitor** + **5.3 UX polish** — alpha-beta arası
  kullanıcı algısı sıçraması.
