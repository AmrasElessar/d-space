# D-Space

**Görmek, anlamak, geri kazanmak.**

Windows üzerinde disk alanını saniyeler içinde haritalayan, ne silinebileceğini bilen ve geri alma garantisi sunan akıllı disk analiz platformu.

## Çekirdek Sözler

- **Hız** — MFT tabanlı tarama motoru ile 1TB SSD < 5 saniye.
- **Zeka** — Her dosyaya 0–100 arası güvenli silme skoru.
- **Geri kazanım** — Direkt silme yok. Tüm silmeler staging klasörüne taşınır, kullanıcı belirlediği pencerede geri alınabilir.

## Durum

Faz 1 implementasyon başlangıcı. Mimari spec v1.4 dondurulmuş — `D-Space-Mimari-v1.4.docx`.

## Tech Stack

- **Backend:** Rust (ntfs, windows-rs, rusqlite, blake3, rayon, tokio)
- **Frontend:** Vue 3 + D3.js
- **Shell:** Tauri 2
- **DB:** SQLite (WAL mode)
- **Hedef:** Windows 10/11 x64 (ARM64 Faz 2)

## Lisans

GPL-3.0-or-later — bkz. [LICENSE](LICENSE).

```
SPDX-License-Identifier: GPL-3.0-or-later
```
