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
Tarih: 2026-05-15 · İlgili bölüm: 34.2, 34.5.4 · Tip: çözüldü (eski tip: ertelendi)

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

### 2026-05-15 Güncellemesi — Plan A seçildi, sprint 2H.3 tamamlandı

`winapi 0.3.9` crate'i `vsbackup` feature ile `IVssBackupComponentsVtbl`
+ `CreateVssBackupComponents` factory'sini sağlıyor — manuel COM vtable
gerekmez. Referans: `docs.rs/winapi/0.3.9 → vsbackup::IVssBackupComponentsVtbl`.

Cargo feature `vss` (default OFF) ile gate'lendi. Default build'e sıfır
etki, opt-in. `Cargo.toml`'da yalnızca `cfg(windows)` target için optional
dependency. Yeni dosyalar:

* `src-tauri/src/locked_file/vss.rs` — düşük seviye COM köprüsü
  (~430 satır): BSTR helpers, COM MTA init, async wait, RawSnapshot create
  /destroy zinciri, `SnapshotProvider` trait (test enjeksiyonu için).
* `src-tauri/src/locked_file/vss_pool.rs` — yüksek seviye pool
  (~360 satır): tek worker thread, per-volume dedupe, reference counting
  + lease renewal (Bölüm 34.5.6), reaper (idle 5 dk → destroy).
* `src-tauri/src/duplicate/scan.rs` — `hash_file_with_retry` zinciri:
  `ERROR_SHARING_VIOLATION` (`raw_os_error == 32`) → VSS pool reader.

**Referans:** worktree `agent-a14d6e5c7ecb9b336`, sprint 2H.3.

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

---

## #004 — VSS context: FILE_SHARE_BACKUP vs BACKUP
Tarih: 2026-05-15 · İlgili bölüm: 34.5.4 · Tip: revize

**Bulgu:** Master spec Bölüm 34.5.4 örnek kodu `SetContext` çağrısını ya
hiç yapmıyor (implicit `VSS_CTX_BACKUP=0`) ya da `VSS_CTX_BACKUP` öneriyor.
`VSS_CTX_BACKUP` ile:

* Writer involvement gerekir — `GatherWriterMetadata` + `PrepareForBackup`
  + `DoSnapshotSet` writer'ları döndürür, "VSS writer didn't respond"
  hataları kullanıcı makinesinde yaygın.
* Persistent + non-auto-release default — leaked snapshot riski geri gelir.
* SQL Server / Exchange writer'lar quiesce sırasında IO duraklatır,
  D-Space hash-time read için gereksiz performans cezası.

D-Space sadece `FILE_READ_DATA` yapacak; writer freeze/thaw'a ihtiyaç
yok (consistency point shorter > 1 saniye olsa bile bizim yeterli).

**Karar:** `SetContext(VSS_CTX_FILE_SHARE_BACKUP = 0x10)` seçildi:

* `VSS_VOLSNAP_ATTR_NO_WRITERS` — writer involvement YOK.
* `VSS_VOLSNAP_ATTR_AUTORECOVER` — auto-recover OFF (read-only,
  yalnızca file-system seviyesi consistency yeter).
* Non-persistent + auto-release — `IVssBackupComponents::Release`'de
  service-side snapshot otomatik kaybolur. Leaked snapshot riski sıfır.

**Etki:** Hash-time path 1-3 sn yerine ~500-800 ms civarında snapshot
açabilir. SQL Server / Outlook writer log gürültüsü yok.

**Referans:**
`src-tauri/src/locked_file/vss.rs` (`SetContext` çağrısı + zincir yorumu);
Bölüm 34.5.4 yorumlarında yapışkan not.

---

## #005 — USN Journal Index katmanı (Everything benzeri)
Tarih: 2026-05-16 · İlgili bölüm: 5 (yeni 5.6) · Tip: revize (spec eksiği)

**Bulgu:** Master spec Bölüm 5.2A Hızlı Mod MFT direkt okumayı tanımlar
(1 TB < 5 sn) ama **persistent index + change stream** mekanizması yok.
Her uygulama açılışında baştan tarama gerekir. `voidtools/Everything`
modeli: `FSCTL_ENUM_USN_DATA` ile baseline MFT enum + `FSCTL_READ_USN_JOURNAL`
ile change stream → açılış sonrası **0 sn render** + real-time file watcher.

**Etki:** D-Space şu an her tarama 5 sn — kullanıcı her açılışta bekler.
USN Index ile:
* Açılış: index load (< 200 ms) + arka planda incremental USN delta sync.
* Search bar substring: in-memory hash → 1M dosyada < 50 ms.
* Real-time dosya watcher: yeni eklenenler/silinenler 1-5 sn lag ile UI'a.

**Yapı:**
* Yeni modül `src-tauri/src/index/` — `usn::enumerate_baseline()`,
  `usn::read_journal_delta()`, `persist::save_index()` / `load_index()`.
* Yeni SQLite tablo: `usn_index (file_ref INTEGER, parent_ref INTEGER,
  name TEXT, usn_id INTEGER, last_seen_unix INTEGER, attrs INTEGER)`.
* Background thread: USN reason mask `USN_REASON_FILE_CREATE |
  FILE_DELETE | RENAME_NEW_NAME | DATA_OVERWRITE` dinler, batch flush 5 sn.
* Frontend: `IndexSearchBar` komponenti (Ctrl+F yeni davranış —
  index üzerinde substring), "Indeksleniyor… N dosya" status badge.

**Karar:** Bölüm 5.6 (USN Index) yeni alt bölüm olarak v1.5 spec
revizyonunda eklenecek. Uygulama tarafı Sprint 3.8'de gelir (v0.2.0-beta).
NTFS-only — ReFS/FAT32/network için fallback Bölüm 5.5'ten gelir.

**Trade-off kabul:**
* USN journal admin gerektirir ama Hızlı Mod zaten admin istiyor — ek UAC yok.
* USN journal disabled volume (nadir) → Tier 2'ye düşer, mevcut fallback yeterli.
* Index storage maliyeti ~50-100 MB / 1 M dosya. `%LOCALAPPDATA%\DSpace\index.db`.
* USN journal dolup wraparound olursa (Bölüm 28.2 gotcha) → full re-enumerate.
  Watermark `next_usn` her flush'ta kaydedilir, miss durumu tespit edilir.

**Referans:** Sprint 3.8 commit'i (henüz açılmadı); Microsoft Learn
`fsctl/fsctl-enum-usn-data` + `fsctl/fsctl-read-usn-journal`.
