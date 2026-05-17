<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# D-Space e2e — Sprint 3.5 (Bölüm 20.4)

Playwright tabanlı uçtan uca testler. İki kanal var:

| Proje | Çalışma yeri | Tauri IPC | Hız | Amaç |
|---|---|---|---|---|
| `web` | Vite dev server | stub (`window.__TAURI_IPC__`) | hızlı | UI flow regresyon, golden path |
| `webdriver` | `tauri-driver` köprüsü | gerçek backend | yavaş | Bundle düzeyinde smoke (release adayı) |

## Yerel koşum

```powershell
pnpm test:e2e:install       # Chromium binary, ilk seferlik
pnpm test:e2e --project=web # Hızlı web kanalı
```

`webdriver` projesi tauri-driver gerektirir:

```powershell
cargo install tauri-driver --locked
pnpm tauri build --debug
pnpm test:e2e --project=webdriver
```

## Smoke senaryoları (`web` kanal stub'ları)

`web` kanalı IPC'yi mocklar — `e2e/fixtures/tauri-mock.ts` her `invoke`
çağrısını deterministik response'larla cevaplar. Smoke testler 5 akışı
kapsar (master spec Bölüm 20.4'ten):

1. **`onboarding.spec.ts`** — 3 slide ileri/geri, mod seçimi, "Başla"
2. **`scan-progress.spec.ts`** — Tara: C: → ScanProgress overlay → drilldown
3. **`view-modes.spec.ts`** — Ctrl+1/2/3/4 görsel mod değişimi
4. **`detail-panel.spec.ts`** — Dosya seçimi → sağ detay panel görünür
5. **`staging-undo.spec.ts`** — Staging gönder → undo → tekrar görünür

Her smoke yalnız UI akışını doğrular; gerçek backend davranışı
`tauri-driver` kanalında ayrı koşulur.

## CI

`.github/workflows/e2e.yml` — şimdilik `continue-on-error: true` kabul
ettirildi. v0.2.0-beta sonrası "must pass" gate'e taşınacak.
