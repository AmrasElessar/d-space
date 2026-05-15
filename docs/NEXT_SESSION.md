<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# D-Space — Sonraki Oturum Yol Planı

> **Son güncelleme:** 2026-05-15 (Sprint 2H.3 VSS pool tamam, `f87979e`)
> **Şu anki sürüm:** `v0.1.0-alpha` (tag çekildi, GitHub Release yayında) + Sprint 2H.3 main'de
> **Hedef:** `v0.2.0-beta` → `v0.3.0` → `v1.0.0-stable`

Bu doküman bir sonraki oturum başlar başlamaz açılacak. Akış:
1. Git pull + son durumu doğrula
2. Aşağıdaki **sıradaki sprint** ile devam et
3. Tamamladığında bu dokümanın "ŞU AN" kısmını güncelle

---

## 🟢 ŞU AN: v0.1.0-alpha → v0.2.0-beta yolundayım

**Sıradaki 3 sprint** (2H.3 tamam — origin'e push'lu):

### Sprint 2H.3 — VSS pool (Bölüm 34 v0.2) ✅ TAMAMLANDI (2026-05-15, `f87979e`)
- **Sonuç:** Plan A — `winapi 0.3.9` crate'i `IVssBackupComponents` + factory'yi sağladı, manuel vtable gerekmedi. Discovery Log #002 → çözüldü, #004 yeni revize (`VSS_CTX_FILE_SHARE_BACKUP` writer-less).
- **Dosyalar:**
  - `src-tauri/src/locked_file/vss.rs` (563 satır) — düşük seviye COM köprüsü, `SnapshotProvider` trait (mock-friendly).
  - `src-tauri/src/locked_file/vss_pool.rs` (511 satır) — worker thread, per-volume dedupe, lease renewal + reaper.
  - `src-tauri/src/duplicate/scan.rs` — `hash_file_with_retry` (`ERROR_SHARING_VIOLATION` → VSS reader).
- **Feature gate:** `vss` (default OFF). Default build sıfır etki.
- **Test:** 9 yeni unit test (hresult mapping, BSTR roundtrip, snapshot_path, pool dedupe, lease drop, reaper eviction + scan integration) — `cargo test --features vss --lib` ile 83/83 geçer.

### Sprint 3.1c — Gerçek üç-kolon (sol volume sidebar)
- **Mevcut:** 2-kolon workspace (sol SnapshotPanel, sağ Duplicate + Detail). 3. kolon henüz yok.
- **Hedef:** Sol sidebar volume listesi (C: D: E: ...) + snapshot timeline. CSS grid `280px 1fr 320px`.
- **Plan:**
  - `volume::list_drives()` Rust API (Win32 `GetLogicalDrives`)
  - `VolumeSidebar.vue` komponenti — drive listesi + her birinin pre-flight bilgisi (file_system + free/total + status)
  - SnapshotPanel sağ kolondan sol kolona taşı (timeline grafik)
  - DuplicatePanel sağ kolonda kalır + Seçili Öğe alta sticky
- **Tahmini boyut:** ~300 satır Rust + ~250 satır Vue + ~50 satır CSS

### Sprint 3.5 — Playwright e2e altyapı
- **Hedef:** `tauri-driver` + `playwright` ile 5 smoke senaryo:
  1. Onboarding akışı (3-slide → mod seç → Başla)
  2. C: taraması başlat → progress overlay → drilldown açılır
  3. Görsel mod değişimi (Ctrl+1/2/3/4)
  4. Sağ panel detayı dolu kalsın (dosya tıkla → detay görünür)
  5. Staging gönder → undo
- **Agent gerekli:** tauri-driver setup ve ilk Playwright config için research agent.
- **CI workflow'a ekle:** windows-latest runner üzerinde e2e job.

### Sprint 3.6 — Tauri updater gerçek
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

**Kalan 3 sprint (3.1c + 3.5 + 3.6) tamamlanınca:** `git tag v0.2.0-beta && git push --tags` → release workflow MSI/NSIS + latest.json üretir.

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

- [ ] 200+ Rust test + 150+ frontend test (şu an 74 default + 9 vss-gated + 10 frontend = 93)
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

## 🚦 Şu anki sprint kararı: **3.1c üç-kolon sidebar** (önerilen)

2H.3 VSS pool tamamlandı, `f87979e` commit'i `origin/main`'de. Sıradaki en
uygun sprint **3.1c**: sol volume sidebar + 3-kolon layout. Düşük risk,
görsel büyük etkili, kullanıcı geri bildirimi ile birebir alakalı.

Alternatif sıra: **3.6 Tauri updater** (production hazırlığı) veya
**3.5 Playwright e2e** (test güvenliği).
