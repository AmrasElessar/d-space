<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# D-Space — Sonraki Oturum Yol Planı

> **Son güncelleme:** 2026-05-16 (Sprint 3.1c üç-kolon sidebar tamam)
> **Şu anki sürüm:** `v0.1.0-alpha` + Sprint 2H.3 (VSS pool) + Sprint 3.1c (3-kolon)
> **Hedef:** `v0.2.0-beta` → `v0.3.0` → `v1.0.0-stable`

Bu doküman bir sonraki oturum başlar başlamaz açılacak. Akış:
1. Git pull + son durumu doğrula
2. Aşağıdaki **sıradaki sprint** ile devam et
3. Tamamladığında bu dokümanın "ŞU AN" kısmını güncelle

---

## 🟢 ŞU AN: v0.1.0-alpha → v0.2.0-beta yolundayım

**Sıradaki 4 sprint** (3.1c tamam — main'e commit'lendi, push bekliyor):

> **Sıralama kararı (2026-05-16):** 3.7 ve 3.8 önce — kullanıcı deneyimi
> doğrudan etkili (canlı görsel + Everything benzeri index). 3.5/3.6 release
> altyapısı, ön kullanıcı geri bildirimi alındıktan sonra eklenir.

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

### Sprint 3.7 — Canlı tarama görseli (Live Sunburst)
- **Mevcut:** `ScanProgress` overlay sadece sayısal (visited/total + ETA + son path). Sunburst/Treemap ancak tarama bitince render eder.
- **Hedef (Bölüm 9.6.5 + 15.4):** Tarama sırasında **canlı dolan mini Sunburst** — kullanıcı diski "doluyor" hisseder.
- **Backend (`src-tauri/`):**
  - `scan/progress.rs` — `ScanProgress` event'ine yeni alan: `Vec<PartialNode>` (root + ilk 2 seviye, top-200 düğüm, aggregate_size). N=10k entry'de bir partial snapshot serialize edilir.
  - `scan/tree.rs` — `build_partial_view(tree, max_depth=2, top_n=200)` API. Tree mutex'i altında lightweight clone, sıralama, kısalt.
  - `scan-progress` event payload kırılma yok — `partial_tree: Option<Vec<PartialNode>>` opsiyonel alan.
- **Frontend (`src/`):**
  - `components/LiveSunburst.vue` (yeni, ~250 satır) — D3 transition tabanlı, her event'te wedge'ler büyür, son taranan path'in altındaki klasör highlight olur.
  - `components/ScanProgress.vue` — sayısal stats sağ tarafta küçülür, sol tarafta `LiveSunburst` 360×360 px.
  - Overlay arka planı semi-transparent, scroll lock korunur.
- **Test:** Rust unit (partial tree top-N + depth limit), frontend mount + event-driven update doğrulama.
- **Boyut tahmini:** ~150 Rust + ~300 Vue/TS + ~80 CSS.
- **Risk:** Düşük. Backend mevcut event'e alan ekliyor; frontend yeni component, App.vue minimal değişir.

### Sprint 3.8 — USN Journal Index katmanı (Everything modeli)
- **Mevcut:** Her açılışta full MFT walk gerekir (5 sn).
- **Hedef (Discovery Log #005, yeni Bölüm 5.6):** Persistent USN index → açılışta < 200 ms load + arka planda incremental delta sync. Search bar substring < 50 ms.
- **Backend (`src-tauri/`):**
  - Yeni modül `index/` — `usn.rs` (`FSCTL_ENUM_USN_DATA` + `FSCTL_READ_USN_JOURNAL` köprü), `persist.rs` (SQLite save/load), `delta.rs` (reason mask filter + apply), `watcher.rs` (background thread + batch flush 5 sn).
  - SQLite migration `0002_usn_index.sql`:
    ```sql
    CREATE TABLE usn_index (
      volume_id TEXT NOT NULL,
      file_ref INTEGER NOT NULL,
      parent_ref INTEGER NOT NULL,
      name TEXT NOT NULL,
      usn_id INTEGER NOT NULL,
      last_seen_unix INTEGER NOT NULL,
      attrs INTEGER NOT NULL,
      PRIMARY KEY (volume_id, file_ref)
    );
    CREATE INDEX idx_usn_name ON usn_index(name);
    CREATE INDEX idx_usn_parent ON usn_index(volume_id, parent_ref);
    CREATE TABLE usn_watermark (
      volume_id TEXT PRIMARY KEY,
      next_usn INTEGER NOT NULL,
      journal_id INTEGER NOT NULL
    );
    ```
  - Wraparound tespiti: `journal_id` değişti → full re-enumerate.
  - Tauri commands: `index_build` (baseline), `index_status`, `index_search(query)`.
- **Frontend (`src/`):**
  - `components/IndexSearchBar.vue` (~200 satır) — Ctrl+F yeni davranış: globe search index üzerinde substring, sonuç listesi instant.
  - `App.vue` — başlangıçta `index_build` arka plan trigger, status badge ("Indexleniyor… %X").
- **Test:** USN reason mask parse, journal wraparound detection, SQLite roundtrip, search latency benchmark.
- **Boyut tahmini:** ~600 Rust + ~250 Vue/TS + ~100 SQL.
- **Risk:** Orta. USN journal admin gerektirir (Hızlı Mod zaten admin); journal disabled fallback path test edilmeli. Wraparound senaryosu unit test ile kapsanmalı.

### Sprint 3.5 — Playwright e2e altyapı (sonra)
- **Hedef:** `tauri-driver` + `playwright` ile 5 smoke senaryo:
  1. Onboarding akışı (3-slide → mod seç → Başla)
  2. C: taraması başlat → progress overlay → drilldown açılır
  3. Görsel mod değişimi (Ctrl+1/2/3/4)
  4. Sağ panel detayı dolu kalsın (dosya tıkla → detay görünür)
  5. Staging gönder → undo + index search
- **Agent gerekli:** tauri-driver setup ve ilk Playwright config için research agent.
- **CI workflow'a ekle:** windows-latest runner üzerinde e2e job.

### Sprint 3.6 — Tauri updater gerçek (sonra)
- **Mevcut:** `tauri.conf.json`'da updater placeholder (`active: false`).
- **Hedef:** `tauri-plugin-updater` install + Ed25519 keygen + GitHub releases `latest.json` otomasyonu + UI dialog.
- **Adımlar (RELEASE_CHECKLIST.md §2'den):**
  ```bash
  pnpm add @tauri-apps/plugin-updater
  cargo add tauri-plugin-updater --manifest-path src-tauri/Cargo.toml
  pnpm tauri signer generate -w ~/.dspace/updater-private.key
  ```
  Private key → GitHub Secret. Public key → `tauri.conf.json`. Release workflow'a `includeUpdaterJson: true` ekle.
- **Test:** v0.2.0-beta tag çek, eski v0.1.0-alpha kurulumdan auto-update çalıştığını doğrula.

**Kalan 4 sprint (3.7 + 3.8 + 3.5 + 3.6) tamamlanınca:** `git tag v0.2.0-beta && git push --tags` → release workflow MSI/NSIS + latest.json üretir.

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

- [ ] 200+ Rust test + 150+ frontend test (şu an 76 default + 9 vss-gated + 16 frontend = 101)
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

## 🚦 Aktif sprint kararı: **3.7 + 3.8 paralel** (worktree, agent)

3.1c üç-kolon sidebar tamamlandı (2026-05-16). v0.2.0-beta'ya 4 sprint:
3.7 (canlı sunburst), 3.8 (USN index), 3.5 (e2e), 3.6 (updater).

**Seçilen sıra (kullanıcı önerisi):** 3.7 ve 3.8 önce — çünkü her ikisi
de doğrudan kullanıcı deneyimi (görsel + arama hızı). 3.7 ile 3.8 dosya
çakışması düşük: 3.7 yalnızca `scan/` + `ScanProgress.vue` +
`LiveSunburst.vue`'a dokunur; 3.8 yalnızca yeni `index/` modülü +
`IndexSearchBar.vue` + SQLite migration. Ortak: `App.vue` birkaç satır.

İki sprint **paralel worktree agent**'larda çalışır:
* `agent-sprint-3-7-live-sunburst` (worktree branch `sprint/3.7-live-sunburst`)
* `agent-sprint-3-8-usn-index` (worktree branch `sprint/3.8-usn-index`)

Tamamlanınca ana branch'e ardışık merge — 3.7 önce (daha küçük yüzey
alanı), sonra 3.8.
