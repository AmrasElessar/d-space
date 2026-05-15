<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# D-Space Implementation Discovery Log

Master mimari (`D-Space-Mimari-v1.4.docx` Bölüm 28) için canlı discovery
defteri. Spec **DONDURULDU** — kod yazılırken çıkan keşifler bu dosyaya
düşer, master doc revize edilmez.

Şablon (Bölüm 28.1):

```
## #NNN — <Başlık>
Tarih: YYYY-MM-DD · İlgili bölüm: <Bölüm X.Y> · Tip: gotcha | revize | ertelendi | iptal
**Bulgu:** ...
**Etki:** ...
**Karar:** ...
**Referans:** <commit hash veya dosya yolu>
```

---

## #001 — VSS `bBackupBootableSystemState` parametresi spec örneğinde yanlış
Tarih: 2026-05-15 · İlgili bölüm: 34.2 · Tip: revize

**Bulgu:** Master spec Bölüm 34.2 örnek kodu `set_backup_state(false, true, VSS_BT_FULL, false)`
yazıyor (`bBackupBootableSystemState=TRUE`, `backupType=VSS_BT_FULL`). Microsoft Learn
([vsbackup/SetBackupState](https://learn.microsoft.com/en-us/windows/win32/api/vsbackup/nf-vsbackup-ivssbackupcomponents-setbackupstate))
ad-hoc volume snapshot için **`bBackupBootableSystemState=FALSE`** ve
**`backupType=VSS_BT_COPY`** öneriyor — system state'i dahil etmek istemiyoruz
ve writer backup history / archive bit'lerini kirletmemeliyiz.

**Etki:** İleride VSS implementasyonunda spec örneği aynen alınırsa
gereksiz system state snapshot maliyeti + writer event log gürültüsü.

**Karar:** VSS sprint'inde `SetBackupState(false, false, VSS_BT_COPY, false)`
kullanılacak. Bölüm 34.2 örneği yalnızca temsili — referans değildir.

---

## #002 — `windows` crate 0.61.3 `IVssBackupComponents`'i sağlamıyor
Tarih: 2026-05-15 · İlgili bölüm: 34.2, 34.5.4 · Tip: ertelendi

**Bulgu:** `windows::Win32::Storage::Vss` modülünde tüm provider/admin/
component/snapshot-mgmt interface'leri var ama **requester-side
`IVssBackupComponents` interface'i ve `CreateVssBackupComponents` factory'si
tamamen YOK**. Sebep: bu API'ler `vsbackup.h` C++ header'da tanımlı,
windows-rs'in otomatik metadata kaynağı (`Windows.Win32.winmd`) onları
dahil etmiyor.

Doğrulama: `C:\Users\engin.okay\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\windows-0.61.3\src\Windows\Win32\Storage\Vss\mod.rs`
içinde `IVssAdmin`, `IVssAsync`, `IVssComponent`, `IVssEnumObject`,
`IVssSnapshotMgmt` vb. mevcut — `IVssBackupComponents` arama sonucu boş.

**Etki:** Bölüm 34.5.4 hash-time snapshot pool + 34.5.5 user drill-down
path'lerinin doğrudan implementasyonu mümkün değil. Üç alternatif:

1. **Manuel COM vtable** (`IVssBackupComponents_Vtbl` struct + 15+ metod
   FFI declaration + IUnknown ref counting). ~400 satır FFI, COM threading
   hassasiyeti, ref count tuzakları, BSTR lifecycle. "Yavaştan ortaya
   çıksın" pacing'ine sığmaz.
2. **Üçüncü taraf crate** (örn. `vsbackup-rs` benzeri). Crates.io'da
   production-ready bir VSS requester crate'i kontrolü v1.5'e bırakıldı.
3. **WMI `Win32_ShadowCopy`** ile snapshot create. ClientAccessible context
   şart (persistent) — Bölüm 34.5.6 non-persistent garantisini bozar,
   leaked snapshot riski geri gelir. Reddedildi.

**Karar:** Bölüm 34 v0.2 VSS pool sprint'i **ertelendi**. Bölüm 34 v0.1
(share-violation probe + RestartManager owner detection) yeterli kapsam
olarak alpha sürümüne dahil. VSS implementasyon kararı sonraki spec
revize turunda (`v1.5` veya `v2.0`) yeniden değerlendirilecek — alternatif
1 ya da 2 seçilecek.

**Referans:** commit `06368db` (Bölüm 34 v0.1); silinen draft
`src-tauri/src/locked_file/vss.rs` (rollback edildi).

---

## #003 — Aktif geliştirme sırası
Tarih: 2026-05-15 · İlgili bölüm: yol haritası · Tip: gotcha

**Bulgu:** Faz 1 implementasyon kullanıcı öncelikleri ile spec'in 37 bölüm
sırasını birebir izlemiyor. Pillar'a göre düzenleniyor:
1. ✅ Bölüm 4.3/4.4/5/14/33.2/12.2/12.3 — hız + altyapı + geri kazanım çekirdek
2. ✅ Bölüm 6 v1 (kural motoru), Bölüm 8 (Time Machine), Bölüm 9.1 (Sunburst),
     Bölüm 9.6 (lazy viewport), Bölüm 7 v0.1 (Blake3 duplicate), Bölüm 34 v0.1 (lock probe)
3. Sıradaki — Bölüm 9.1 diğer 3 mod (Treemap/Bubble/Timeline), Bölüm 12.4
     permanent delete, Bölüm 20/21 test + CI/installer, Bölüm 6.2 eksik kurallar.

**Karar:** Discovery Log her sprint sonunda dolacak. `DISCOVERY_LOG.md`
master spec'e referans, master spec değişmeden bu dosya canlı kalır.
