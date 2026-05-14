# D-Space

**Görmek, anlamak, geri kazanmak.** · **See, understand, reclaim.**

Windows için akıllı disk analiz ve geri kazanım platformu / Smart disk analysis and recovery platform for Windows.

[![License: GPL v3+](https://img.shields.io/badge/License-GPL_v3+-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6)
![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-24c8db)
![Vue](https://img.shields.io/badge/Vue-3.5-42b883)
![Spec](https://img.shields.io/badge/spec-v1.4_DONDURULDU-1e3a8a)

---

<details open>
<summary><b>🇹🇷 Türkçe</b></summary>

## Vizyon

D-Space, Windows üzerinde disk alanını saniyeler içinde haritalayan, ne silinebileceğini bilen ve geri alma garantisi sunan akıllı disk analiz platformudur.

## Üç Çekirdek Söz

- **Hız** — MFT tabanlı tarama motoru ile 1TB SSD < 5 saniye hedefi.
- **Zeka** — Her dosyaya 0–100 arası güvenli silme skoru.
- **Geri kazanım** — Direkt silme yok. Tüm silmeler staging klasörüne atomik MOVE'lanır, 24 saatlik pencerede tek tıkla geri alınabilir.

## Beş Pillar (Bölüm 3)

| # | Pillar | Durum |
|---|--------|-------|
| 1 | **Hız** — MFT motoru | ✅ MFT probe + full walk + FindFirstFile fallback |
| 2 | **Görsel zarafet** — sunburst/treemap/bubble/timeline | ✅ Sunburst donut + drilldown + breadcrumb |
| 3 | **Zeka katmanı** — 50+ kural + ML tier | ✅ 33 kural, 4 renk tier (kırmızı/sarı/yeşil/mavi); ML v2 |
| 4 | **Zaman boyutu** — günlük snapshot + delta | ✅ Capture + delta (added/removed/grew/shrunk top 10) |
| 5 | **Geri kazanım garantisi** — staging + undo | ✅ Same-volume + cross-volume two-phase commit + WAL |

## Mimari İlkeler

- **Single Source of Truth** (Bölüm 4.4) — `Arc<ScanTree>` Rust tarafında tek sahip. Vue sadece `NodeId` referansı tutar.
- **Üç Katmanlı Yetki Stratejisi** (Bölüm 5.2A) — MFT Service → admin raw volume → FindFirstFile fallback. UAC ilk açılışta sorulmaz.
- **VSS Hot-Path İzolasyonu** (Bölüm 34.5.1) — VSS scan sırasında ASLA çalışmaz, sadece duplicate hash / drill-down'da.
- **Cross-Volume Two-Phase Commit** (Bölüm 12.3 v1.4 fix) — `.dspace_tmp` + WAL + Blake3 hash verify + atomik rename. Crash sonrası otomatik recovery.
- **Lazy Viewport Query** (Bölüm 9.6) — Vue hiçbir zaman tam ağaca sahip olmaz, sadece görünen ~200 düğümlük pencereyi ister.

## Tech Stack

- **Backend:** Rust 1.80+ — `ntfs`, `windows-rs`, `rusqlite` + `rusqlite_migration`, `blake3`, `tokio`, `tracing`, `thiserror`, `dirs`
- **Frontend:** Vue 3 + TypeScript + Vite — SVG sunburst hand-rolled (d3 yok)
- **Shell:** Tauri 2.x
- **DB:** SQLite (WAL mode, `busy_timeout=5000`, `synchronous=NORMAL`)
- **Hedef:** Windows 10/11 x64 — ARM64 Faz 2

## Çalıştırma

```powershell
# Bağımlılıklar
pnpm install

# Geliştirme — pencereyi açar (Vite dev server + Tauri)
pnpm tauri dev

# Yapım — debug binary (src-tauri/target/debug/dspace.exe)
cd src-tauri
cargo build

# Test
cargo test --lib            # 36 birim test geçer
pnpm build                  # vue-tsc + vite build
```

## Durum

**Faz 1 implementasyon devam ediyor.** Mimari spec v1.4 **dondurulmuş** (`D-Space-Mimari-v1.4.docx`, 37 bölüm). Spec'e yeni revizyon eklenmez; kod yazılırken çıkan keşifler Bölüm 28 Discovery Log'a düşer.

13 commit · 36/36 test geçiyor · 5/5 pillar canlı.

## Lisans

**GPL-3.0-or-later** — bkz. [LICENSE](LICENSE). Her kaynak dosya başlığında:

```
SPDX-License-Identifier: GPL-3.0-or-later
```

</details>

<details>
<summary><b>🇬🇧 English</b></summary>

## Vision

D-Space is a smart disk analyzer for Windows that maps disk usage in seconds, knows what's safe to delete, and guarantees recovery.

## Three Core Promises

- **Speed** — MFT-based scanner targeting 1TB SSD in under 5 seconds.
- **Intelligence** — Every file gets a 0–100 safe-to-delete score.
- **Recovery** — No direct deletion. All deletions are atomically moved to a staging folder and can be undone in a single click within a 24-hour window.

## Five Pillars (Section 3)

| # | Pillar | Status |
|---|--------|--------|
| 1 | **Speed** — MFT engine | ✅ MFT probe + full walk + FindFirstFile fallback |
| 2 | **Visual elegance** — sunburst/treemap/bubble/timeline | ✅ Sunburst donut + drilldown + breadcrumb |
| 3 | **Intelligence layer** — 50+ rules + ML tier | ✅ 33 rules, 4-color tier (red/yellow/green/blue); ML in v2 |
| 4 | **Time dimension** — daily snapshot + delta | ✅ Capture + delta (added/removed/grew/shrunk top 10) |
| 5 | **Recovery guarantee** — staging + undo | ✅ Same-volume + cross-volume two-phase commit + WAL |

## Architectural Principles

- **Single Source of Truth** (Section 4.4) — `Arc<ScanTree>` lives in Rust. Vue only holds `NodeId` references.
- **Three-Tier Privilege Strategy** (Section 5.2A) — MFT Service → admin raw volume → FindFirstFile fallback. UAC is never prompted on first launch.
- **VSS Hot-Path Isolation** (Section 34.5.1) — VSS NEVER runs during scan; only for duplicate hashing and drill-down.
- **Cross-Volume Two-Phase Commit** (Section 12.3 v1.4 fix) — `.dspace_tmp` + WAL + Blake3 hash verify + atomic rename. Automatic crash recovery on startup.
- **Lazy Viewport Query** (Section 9.6) — Vue never owns the full tree, only requests visible ~200-node windows.

## Tech Stack

- **Backend:** Rust 1.80+ — `ntfs`, `windows-rs`, `rusqlite` + `rusqlite_migration`, `blake3`, `tokio`, `tracing`, `thiserror`, `dirs`
- **Frontend:** Vue 3 + TypeScript + Vite — hand-rolled SVG sunburst (no d3)
- **Shell:** Tauri 2.x
- **DB:** SQLite (WAL mode, `busy_timeout=5000`, `synchronous=NORMAL`)
- **Target:** Windows 10/11 x64 — ARM64 in Phase 2

## Running

```powershell
# Dependencies
pnpm install

# Development — opens the window (Vite dev server + Tauri)
pnpm tauri dev

# Build — debug binary (src-tauri/target/debug/dspace.exe)
cd src-tauri
cargo build

# Tests
cargo test --lib            # 36 unit tests pass
pnpm build                  # vue-tsc + vite build
```

## Status

**Phase 1 implementation in progress.** Architectural spec v1.4 is **frozen** (`D-Space-Mimari-v1.4.docx`, 37 sections). No new revisions to the spec; discoveries during coding go to Section 28 Discovery Log.

13 commits · 36/36 tests passing · 5/5 pillars live.

## License

**GPL-3.0-or-later** — see [LICENSE](LICENSE). Every source file carries:

```
SPDX-License-Identifier: GPL-3.0-or-later
```

</details>

---

📐 Master architecture document: `D-Space-Mimari-v1.4.docx` (in repo root)
🌐 Domain: `dspace.app` (planlanan / planned)
