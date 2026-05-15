<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# D-Space Changelog

Spec v1.4 dondurulmuş. Implementation farkı `docs/DISCOVERY_LOG.md`'de.

## [v0.1.0-alpha] — 2026-05-15

İlk public alpha. **Unsigned** MSI/NSIS — Windows SmartScreen "publisher
unknown" uyarısı verecek (Bölüm 18.2 EV cert v0.2 sprint'inde).

### Yapıldı

**Pillar 1 — Hız (Bölüm 5):**
- MFT direkt okuma (`ntfs` crate) — admin ile 1 TB hedef <5 sn
- FindFirstFile fallback — non-elevated mod, her durum çalışır
- Üç katmanlı yetki stratejisi (Bölüm 5.2A K1+K2)
- Volume pre-flight (Bölüm 33.2 Katman 0): ReFS/Remote/Removable/CdRom uyarıları

**Pillar 2 — Görsel zarafet (Bölüm 9.1):**
- Sunburst donut (hand-rolled SVG)
- Treemap (squarified Bruls/Huijsen/van Wijk 2000)
- Bubble pack (Vogel spiral + 80-iter force relax)
- Timeline (mtime ekseni + Y-relax, log scale > 1 yıl)
- Lazy viewport-aware query (Bölüm 9.6): ~200 düğüm/window

**Pillar 3 — Zeka katmanı (Bölüm 6):**
- 63 built-in kural — dev cache / sistem koruma / cloud sync / VM disk
- Skor tier renkleri: 0-30 kırmızı, 31-60 sarı, 61-85 yeşil, 86-100 mavi
- Kullanıcı tanımlı kurallar — UI editör + DB persist + runtime merge (Bölüm 6.4)
- `RulesScorer` concrete impl + SQLite `ml_scores` cache (Bölüm 6.5.2)
- TFLite altyapı `ml-tflite` feature gate (model v0.3'te)

**Pillar 4 — Zaman boyutu (Bölüm 8):**
- Snapshot capture (dir-only v0.1)
- Delta hesaplama (added/removed/grew/shrunk top-10)

**Pillar 5 — Geri kazanım garantisi (Bölüm 12):**
- Same-volume staging + 24h undo
- Cross-volume two-phase commit + WAL recovery (Bölüm 12.3 v1.4 fix)
- Conflict resolution dialog (Bölüm 12.2.4): Overwrite/Rename/KeepBoth/Cancel
- Permanent delete + forensic ledger (Bölüm 12.4) — çift onay (file name typed)
- Lazy expiry cleanup (Bölüm 12.2.1-2.3) — rate-limited, AUTO_THRESHOLD=100

**Duplicate detector (Bölüm 7) v0.1:**
- Blake3 streaming hash
- Boyut bucket → hash bucket pipeline
- Reclaimable bytes raporu

**Locked file (Bölüm 34) v0.1:**
- CreateFileW share-violation probe
- Windows Restart Manager owner detection (PID + app name)
- Scan-time hot-path izolasyonu (Bölüm 34.5.1)
- VSS pool **ertelendi** — Discovery Log #002 (windows-rs 0.61
  `IVssBackupComponents` eksik, v0.2'de albertony/vss port)

**UI/UX (Bölüm 15):**
- Progressive Disclosure 3-seviye (özet · expand · advanced)
- Klavye-first: Ctrl+R, Ctrl+1-4, Backspace, Delete, Ctrl+Z, Ctrl+F, Ctrl+?
- Onboarding 3-slide + Hızlı/Standart mod seçimi (Bölüm 15.3 + 37)
- Sistem tray ikon + menü (Bölüm 13.1)
- Çakışma/permanent delete modal'ları

**Çoklu dil (Bölüm 19):**
- vue-i18n v10 + TR/EN locale + dil toggle chip
- Onboarding tamamen çevrilmiş, ana akış çevrilmiş

**Altyapı:**
- SQLite (Bölüm 14): 9 tablo, WAL mode, 2 migration
- Settings KV API (onboarding/dil/telemetry tercihleri)
- Perf telemetrisi yerel (son 20 örnek, gönderim YOK)
- Telemetry opt-in flag (default kapalı, Bölüm 18.3)

**v2 trait rezervasyonları (Bölüm 26):**
- 5 trait stub: MlSafeDeleteScorer · NetworkShareScanner ·
  CrossPlatformVolumeReader · Plugin · CloudBackupIntegration

**Build + dağıtım:**
- GitHub Actions CI: fmt + clippy `-D warnings` + cargo test + pnpm build + vitest
- Release workflow: tag → tauri-action → MSI + NSIS + GitHub Release

**Test:**
- 69 Rust unit/integration + 10 frontend Vitest = **79 test**

**Dokümantasyon:**
- README iki dilli (TR/EN) D Brand template
- REPO_STANDARDS.md
- docs/DISCOVERY_LOG.md (#001-003)
- docs/THREAT_MODEL.md (Bölüm 27, 5 aktör + 8 yüzey + T1-T5)
- docs/RELEASE_CHECKLIST.md (Bölüm 18.2 + 21.4 + 35)

### Bilinen sınırlar (v0.2 sprint'lerine)

- **Kod imzasız** — SmartScreen "publisher unknown" uyarısı (EV cert v0.2)
- **VSS pool yok** — kilitli dosyalar hash retry yapamaz (Discovery Log #002)
- **ML inference yok** — model dosyası v0.3'te eğitilecek
- **Auto-updater pasif** — Ed25519 key + endpoint v0.2'de
- **Üç-kolon layout yok** — tek-kolon (v0.2 Faz 3.1)
- **Light tema yok** — sadece dark (v0.2 Faz 3.2)
- **Tray polling/auto-clean yok** — sadece launcher/quit (v0.2 Faz 3.4)
- **Win32 reparse tag detection yok** — cloud placeholder pattern-bazlı
- **Playwright e2e yok** — sadece Vitest unit (v0.2 Faz 3.5)

### Lisans

GPL-3.0-or-later. Doküman Bölüm 22.4 MIT öneriyordu, 2026-05-14
override edildi.
